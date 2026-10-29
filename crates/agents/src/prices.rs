//! Published API rates, refreshed separately from local telemetry. No embedded price table.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const SOURCE: &str = "https://models.dev/api.json";
pub const MAX_AGE: i64 = 24 * 60 * 60;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Catalog {
    pub checked_at: i64,
    pub etag: Option<String>,
    pub models: BTreeMap<String, BTreeMap<String, Price>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Price {
    base: Rate,
    tiers: Vec<(u64, Rate)>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct Rate {
    input: f64,
    output: f64,
    cache_read: Option<f64>,
    cache_write: Option<f64>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tokens {
    pub input: u64,
    pub cached: u64,
    pub written: u64,
    pub output: u64,
}

impl Tokens {
    pub fn read(value: &Value) -> Option<Self> {
        let tokens = Self {
            input: value["input_tokens"].as_u64()?,
            output: value["output_tokens"].as_u64()?,
            cached: value["cached_input_tokens"].as_u64().unwrap_or(0),
            written: value["cache_write_input_tokens"].as_u64().unwrap_or(0),
        };
        (tokens.cached.checked_add(tokens.written)? <= tokens.input).then_some(tokens)
    }

    pub fn delta(self, previous: Self) -> Option<Self> {
        Some(Self {
            input: self.input.checked_sub(previous.input)?,
            output: self.output.checked_sub(previous.output)?,
            cached: self.cached.checked_sub(previous.cached)?,
            written: self.written.checked_sub(previous.written)?,
        })
    }
}

impl Rate {
    fn parse(value: &Value, base: Option<&Self>) -> Option<Self> {
        let read = |key: &str, fallback: Option<f64>| match value.get(key) {
            Some(v) => v
                .as_f64()
                .filter(|n| n.is_finite() && (0.0..=100_000.0).contains(n)),
            None => fallback,
        };
        let input = read("input", base.map(|b| b.input))?;
        let output = read("output", base.map(|b| b.output))?;
        // Missing cache rates stay unknown; a cached token must never be priced as free.
        let cache_read = read("cache_read", base.and_then(|b| b.cache_read));
        let cache_write = read("cache_write", base.and_then(|b| b.cache_write));
        Some(Self {
            input,
            output,
            cache_read,
            cache_write,
        })
    }

    fn valid(&self) -> bool {
        [
            Some(self.input),
            Some(self.output),
            self.cache_read,
            self.cache_write,
        ]
        .into_iter()
        .flatten()
        .all(|n| n.is_finite() && (0.0..=100_000.0).contains(&n))
    }
}

impl Catalog {
    pub fn from_models_dev(value: &Value, checked_at: i64, etag: Option<String>) -> Result<Self, String> {
        let mut models = BTreeMap::new();
        for (provider, entry) in value.as_object().ok_or("Invalid price catalog")? {
            let Some(rows) = entry["models"].as_object() else {
                continue;
            };
            let mut prices = BTreeMap::new();
            for (model, row) in rows {
                let cost = &row["cost"];
                let Some(base) = Rate::parse(cost, None) else {
                    continue;
                };
                let mut tiers = Vec::new();
                let mut valid = true;
                if let Some(entries) = cost["tiers"].as_array() {
                    for tier in entries {
                        if tier["tier"]["type"] != "context" {
                            continue;
                        }
                        let (Some(size), Some(rate)) =
                            (tier["tier"]["size"].as_u64(), Rate::parse(tier, Some(&base)))
                        else {
                            valid = false;
                            break;
                        };
                        tiers.push((size, rate));
                    }
                }
                if !valid {
                    continue;
                }
                tiers.sort_by_key(|(size, _)| *size);
                prices.insert(model.clone(), Price { base, tiers });
            }
            if !prices.is_empty() {
                models.insert(provider.clone(), prices);
            }
        }
        let catalog = Self {
            checked_at,
            etag,
            models,
        };
        if !catalog.valid() {
            return Err("Empty or invalid price catalog".into());
        }
        Ok(catalog)
    }

    pub fn valid(&self) -> bool {
        self.checked_at > 0
            && !self.models.is_empty()
            && self.models.values().all(|models| {
                models
                    .values()
                    .all(|price| price.base.valid() && price.tiers.iter().all(|(_, rate)| rate.valid()))
            })
    }

    pub fn fresh(&self, now: i64) -> bool {
        (0..=MAX_AGE).contains(&(now - self.checked_at))
    }

    /// Resolve only an exact model with one catalog provider; never guess among vendors.
    pub fn provider_for(&self, model: &str) -> Option<&str> {
        let mut matches = self
            .models
            .iter()
            .filter(|(_, models)| models.contains_key(model));
        let (provider, _) = matches.next()?;
        matches.next().is_none().then_some(provider.as_str())
    }

    /// An aggregate has no per-call context sizes. Tiered rates cannot be inferred from it.
    pub fn estimate_total(&self, provider: &str, model: &str, tokens: Tokens) -> Option<f64> {
        self.models
            .get(provider)?
            .get(model)?
            .tiers
            .is_empty()
            .then_some(())?;
        self.estimate(provider, model, tokens, 0)
    }

    /// Current standard API equivalent. A subscription's charge or a negotiated price is separate.
    pub fn estimate(&self, provider: &str, model: &str, tokens: Tokens, context_input: u64) -> Option<f64> {
        let price = self.models.get(provider)?.get(model)?;
        let rate = price
            .tiers
            .iter()
            .rev()
            .find(|(above, _)| context_input > *above)
            .map_or(&price.base, |(_, rate)| rate);
        let fresh = tokens
            .input
            .checked_sub(tokens.cached.checked_add(tokens.written)?)?;
        let cached = if tokens.cached == 0 {
            0.0
        } else {
            tokens.cached as f64 * rate.cache_read?
        };
        let written = if tokens.written == 0 {
            0.0
        } else {
            tokens.written as f64 * rate.cache_write?
        };
        Some(
            (fresh as f64 * rate.input + tokens.output as f64 * rate.output + cached + written) / 1_000_000.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn current_catalog_prices_cache_and_context_tiers_without_guessing_missing_rates() {
        let value = json!({"vendor":{"models":{"unit":{"cost":{"input":2,"output":10,"cache_read":0.1,
            "tiers":[{"tier":{"type":"context","size":1000},"input":4,"output":15,"cache_read":0.2}]}},
            "missing":{"cost":{"input":1}},"invalid":{"cost":{"input":-1,"output":2}}}}});
        let book = Catalog::from_models_dev(&value, 1000, Some("v1".into())).unwrap();
        let tokens = Tokens {
            input: 100,
            cached: 50,
            output: 20,
            ..Default::default()
        };
        assert!((book.estimate("vendor", "unit", tokens, 100).unwrap() - 0.000305).abs() < 1e-12);
        assert!((book.estimate("vendor", "unit", tokens, 1001).unwrap() - 0.00051).abs() < 1e-12);
        assert_eq!(book.estimate("other", "unit", tokens, 100), None);
        assert_eq!(book.estimate("vendor", "missing", tokens, 100), None);
        assert_eq!(book.estimate("vendor", "invalid", tokens, 100), None);
        assert!(book.fresh(1000 + MAX_AGE));
        assert!(!book.fresh(1001 + MAX_AGE));
        assert!(!book.fresh(999));
        let updated = json!({"vendor":{"models":{"unit":{"cost":{"input":3,"output":12,"cache_read":0.2}}}}});
        let updated = Catalog::from_models_dev(&updated, 1100, None).unwrap();
        assert!(
            updated.estimate("vendor", "unit", tokens, 100) > book.estimate("vendor", "unit", tokens, 100)
        );
        let no_cache = Catalog::from_models_dev(
            &json!({"vendor":{"models":{"unit":{"cost":{"input":2,"output":10}}}}}),
            1000,
            None,
        )
        .unwrap();
        assert_eq!(no_cache.estimate("vendor", "unit", tokens, 100), None);
        assert!(Catalog::from_models_dev(&json!({}), 1000, None).is_err());
        assert!(Tokens::read(&json!({"input_tokens":1,"cached_input_tokens":2,"output_tokens":1})).is_none());
    }
}
