//! Provenance and cumulative consumption are separate from current context occupancy.
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum TimeKind {
    Execution,
    ModelResponse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum CostSource {
    Agent,
    AgentEstimate,
    Catalog,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct MetricInfo {
    pub model: Option<String>,
    pub provider: Option<String>,
    pub price_provider: Option<String>,
    pub time_kind: Option<TimeKind>,
    pub cost_source: Option<CostSource>,
    #[cfg_attr(test, ts(as = "Option<f64>"))]
    pub input_tokens: Option<u64>,
    #[cfg_attr(test, ts(as = "Option<f64>"))]
    pub output_tokens: Option<u64>,
    #[cfg_attr(test, ts(as = "Option<f64>"))]
    pub cached_tokens: Option<u64>,
    #[cfg_attr(test, ts(as = "Option<f64>"))]
    pub cache_write_tokens: Option<u64>,
    pub premium_requests: Option<f64>,
    pub service_credits: Option<f64>,
}

impl MetricInfo {
    /// Separate channels may report only one metric; missing metadata never erases another.
    pub fn merge(&mut self, incoming: Self) {
        if incoming.model.is_some() && incoming.model != self.model {
            self.provider = None;
            self.price_provider = None;
        }
        macro_rules! take { ($($f:ident),*) => { $(if incoming.$f.is_some() { self.$f = incoming.$f; })* }; }
        take!(
            model,
            provider,
            price_provider,
            time_kind,
            cost_source,
            input_tokens,
            output_tokens,
            cached_tokens,
            cache_write_tokens,
            premium_requests,
            service_credits
        );
    }
}
