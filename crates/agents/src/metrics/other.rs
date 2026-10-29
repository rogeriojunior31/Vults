//! Read numeric records for an already-known session; never discover sessions from history.
use super::*;

fn safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

pub(super) fn file(dirs: &crate::Dirs, key: &SessionKey) -> Option<PathBuf> {
    let (root, id, suffix, nested) = if key.agent == AgentKind::Copilot {
        (
            dirs.agent("COPILOT_HOME", ".copilot").join("session-state"),
            key.session_id.as_str(),
            "events.jsonl",
            false,
        )
    } else if let Some(id) = key.session_id.strip_prefix("kimi/") {
        (
            dirs.agent("KIMI_CODE_HOME", ".kimi-code").join("sessions"),
            id,
            "agents/main/wire.jsonl",
            true,
        )
    } else {
        let id = key.session_id.strip_prefix("droid/")?;
        (
            dirs.var("FACTORY_HOME_OVERRIDE")
                .unwrap_or(dirs.home())
                .join(".factory/sessions"),
            id,
            "",
            true,
        )
    };
    if !safe_id(id) {
        return None;
    }
    let root = root.canonicalize().ok()?;
    let candidate = |base: &Path| {
        if suffix.is_empty() {
            base.join(format!("{id}.settings.json"))
        } else {
            base.join(id).join(suffix)
        }
    };
    let safe = |path: PathBuf| {
        path.canonicalize()
            .ok()
            .filter(|p| p.starts_with(&root) && p.is_file())
    };
    if let Some(path) = safe(candidate(&root)) {
        return Some(path);
    }
    if nested {
        for entry in std::fs::read_dir(&root).ok()?.filter_map(Result::ok) {
            if let Some(path) = safe(candidate(&entry.path())) {
                return Some(path);
            }
        }
    }
    None
}

pub(super) fn droid(path: &Path) -> Option<Snapshot> {
    let mut raw = Vec::new();
    File::open(path)
        .ok()?
        .take(256 * 1024 + 1)
        .read_to_end(&mut raw)
        .ok()?;
    if raw.len() > 256 * 1024 {
        return None;
    }
    let data: Value = serde_json::from_slice(&raw).ok()?;
    let usage = &data["tokenUsage"];
    // Native settings carry cumulative usage, not current context or per-call duration.
    Some(Snapshot {
        info: MetricInfo {
            model: data["model"].as_str().map(str::to_string),
            input_tokens: usage["input"].as_u64().and_then(|input| {
                input
                    .checked_add(usage["cacheRead"].as_u64()?)?
                    .checked_add(usage["cacheCreation"].as_u64()?)
            }),
            output_tokens: usage["output"].as_u64(),
            cached_tokens: usage["cacheRead"].as_u64(),
            cache_write_tokens: usage["cacheCreation"].as_u64(),
            service_credits: usage["factoryCredits"]
                .as_f64()
                .filter(|n| n.is_finite() && *n >= 0.0),
            ..MetricInfo::default()
        },
        ..Snapshot::default()
    })
}

pub(super) fn kimi_usage(record: &Value) -> Option<Value> {
    if record["type"] != "usage.record" {
        return None;
    }
    let usage = &record["usage"];
    let input = usage["inputOther"]
        .as_u64()?
        .checked_add(usage["inputCacheRead"].as_u64()?)?
        .checked_add(usage["inputCacheCreation"].as_u64()?)?;
    Some(
        serde_json::json!({"type":"assistant","model":record["model"],"context_report":record["usageScope"]!="session","context_reset":record["usageScope"]=="session","usageMetadata": {
            "promptTokenCount":input, "candidatesTokenCount":usage["output"],
            "cachedContentTokenCount":usage["inputCacheRead"], "cacheCreationTokenCount":usage["inputCacheCreation"]
        }}),
    )
}

/// Copilot's shutdown ledger is cumulative across resumes. Replace it; never add shutdowns.
/// `requests.cost`/premium requests are credits, not USD. Never convert them to dollars.
pub(super) fn copilot(record: &Value, prices: Option<&Catalog>) -> Option<Snapshot> {
    if record["type"] != "session.shutdown" || record.get("agentId").is_some() {
        return None;
    }
    let data = &record["data"];
    let models = data["modelMetrics"].as_object()?;
    let mut input = 0u64;
    let mut output = 0u64;
    let mut cached = 0u64;
    let mut written = 0u64;
    let mut cost = Some(0.0);
    for (model, metric) in models {
        let usage = &metric["usage"];
        let tokens = Tokens {
            input: usage["inputTokens"].as_u64()?,
            output: usage["outputTokens"].as_u64()?,
            cached: usage["cacheReadTokens"].as_u64()?,
            written: usage["cacheWriteTokens"].as_u64()?,
        };
        input = input.checked_add(tokens.input)?;
        output = output.checked_add(tokens.output)?;
        cached = cached.checked_add(tokens.cached)?;
        written = written.checked_add(tokens.written)?;
        cost = cost
            .zip(prices.and_then(|p| p.estimate_total(p.provider_for(model)?, model, tokens)))
            .map(|(a, b)| a + b);
    }
    Some(Snapshot {
        info: MetricInfo {
            model: data["currentModel"].as_str().map(str::to_string),
            time_kind: Some(TimeKind::ModelResponse),
            cost_source: Some(CostSource::Catalog),
            input_tokens: (!models.is_empty()).then_some(input),
            output_tokens: (!models.is_empty()).then_some(output),
            cached_tokens: (!models.is_empty()).then_some(cached),
            cache_write_tokens: (!models.is_empty()).then_some(written),
            premium_requests: data["totalPremiumRequests"]
                .as_f64()
                .filter(|n| n.is_finite() && *n >= 0.0),
            ..MetricInfo::default()
        },
        estimated_cost_usd: prices.filter(|_| !models.is_empty()).and(cost),
        cost_estimated: true,
        price_checked_at: prices.map(|p| p.checked_at),
        elapsed_ms: data["totalApiDurationMs"].as_u64(),
        context_tokens: data["currentTokens"].as_u64(),
        ..Snapshot::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn persisted_copilot_kimi_and_droid_records_keep_native_units_and_session_identity() {
        let root = std::env::temp_dir().join(format!("vults-other-metrics-{}", std::process::id()));
        let copilot = root.join(".copilot/session-state/c1/events.jsonl");
        let kimi = root.join(".kimi-code/sessions/work/k1/agents/main/wire.jsonl");
        let droid = root.join(".factory/sessions/work/d1.settings.json");
        for path in [&copilot, &kimi, &droid] {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        }
        let shutdown=json!({"type":"session.shutdown","data":{"totalApiDurationMs":1250,"currentTokens":50,"currentModel":"unit","totalPremiumRequests":2,"modelMetrics":{"unit":{"requests":{"cost":99},"usage":{"inputTokens":100,"outputTokens":12,"cacheReadTokens":20,"cacheWriteTokens":10}}}}}).to_string()+"\n";
        std::fs::write(&copilot, shutdown.repeat(2)).unwrap();
        let usage=json!({"type":"usage.record","model":"unit","usage":{"inputOther":10,"inputCacheRead":20,"inputCacheCreation":5,"output":3}}).to_string()+"\n";
        std::fs::write(&kimi, usage.clone() + "{\"type\":\"forked\"}\n" + &usage).unwrap();
        std::fs::write(
            &droid,
            json!({"model":"unit","tokenUsage":{"input":10,"output":3,"cacheRead":20,"cacheCreation":5}})
                .to_string(),
        )
        .unwrap();
        let mut reader = Reader::default();
        reader.set_prices(Some(Arc::new(Catalog::from_models_dev(&json!({"vendor":{"models":{"unit":{"cost":{"input":2,"output":10,"cache_read":0.1,"cache_write":3}}}}}),epoch(),None).unwrap())));
        let dirs = crate::Dirs::home_only(&root);
        let keys = [
            SessionKey {
                agent: AgentKind::Copilot,
                session_id: "c1".into(),
            },
            SessionKey {
                agent: AgentKind::Other,
                session_id: "kimi/k1".into(),
            },
            SessionKey {
                agent: AgentKind::Other,
                session_id: "droid/d1".into(),
            },
        ];
        let rows = reader.read(&dirs, &root, &keys);
        assert_eq!(rows.len(), 3);
        let copilot = &rows[0].1;
        assert_eq!(copilot.elapsed_ms, Some(1250));
        assert_eq!(copilot.info.premium_requests, Some(2.0));
        assert!(
            (copilot.estimated_cost_usd.unwrap() - 0.000292).abs() < 1e-12,
            "Credit cost must never become USD, nor cumulative shutdowns be summed"
        );
        assert_eq!(
            rows[1].1.info.input_tokens,
            Some(35),
            "Inherited fork records are excluded"
        );
        assert_eq!(rows[1].1.info.cache_write_tokens, Some(5));
        assert!((rows[1].1.estimated_cost_usd.unwrap() - 0.000067).abs() < 1e-12);
        assert_eq!(rows[2].1.info.input_tokens, Some(35));
        assert_eq!(rows[2].1.context_tokens, None);
        assert_eq!(rows[2].1.cost_usd, None);
        assert_eq!(
            reader.read(&dirs, &root, &keys),
            rows,
            "Polling unchanged files is idempotent"
        );
        let mut catalog = reader.prices.as_deref().unwrap().clone();
        catalog.checked_at = epoch() - crate::prices::MAX_AGE - 1;
        reader.set_prices(Some(Arc::new(catalog.clone())));
        let stale = reader.read(&dirs, &root, &keys);
        assert!(stale[0].1.estimated_cost_usd.is_none());
        assert!(stale[1].1.estimated_cost_usd.is_none());
        catalog.checked_at = epoch();
        reader.set_prices(Some(Arc::new(catalog)));
        let fresh = reader.read(&dirs, &root, &keys);
        assert!(fresh[0].1.estimated_cost_usd.is_some());
        assert!(fresh[1].1.estimated_cost_usd.is_some());
        assert!(
            file(
                &dirs,
                &SessionKey {
                    agent: AgentKind::Other,
                    session_id: "droid/../d1".into()
                }
            )
            .is_none()
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
