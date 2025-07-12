// src/metrics/collector.rs
use prometheus::{CounterVec, Encoder, HistogramVec, Opts, Registry, TextEncoder};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct MetricsCollector {
    registry: Registry,
    request_count: CounterVec,
    response_time: HistogramVec,
    data: Arc<Mutex<()>>, // Dummy data for thread safety marker
}

impl MetricsCollector {
    pub fn new() -> Self {
        let registry = Registry::new();

        let request_count_opts = Opts::new("requests_total", "Total number of requests processed")
            .namespace("gemini_proxy")
            .subsystem("proxy");
        let request_count = CounterVec::new(request_count_opts, &["api_key_id"]).unwrap();

        let response_time_opts = Opts::new("response_time_seconds", "Request response time")
            .namespace("gemini_proxy")
            .subsystem("proxy");
        let response_time = HistogramVec::new(response_time_opts.into(), &["status_code"]).unwrap();

        registry.register(Box::new(request_count.clone())).unwrap();
        registry.register(Box::new(response_time.clone())).unwrap();

        Self {
            registry,
            request_count,
            response_time,
            data: Arc::new(Mutex::new(())),
        }
    }

    pub async fn increment_request_count(&self, api_key_id: &str) {
        let _lock = self.data.lock().unwrap();
        self.request_count.with_label_values(&[api_key_id]).inc();
    }

    pub async fn record_response(&self, status: u16, duration: Duration) {
        let _lock = self.data.lock().unwrap();
        self.response_time
            .with_label_values(&[&status.to_string()])
            .observe(duration.as_secs_f64());
    }

    pub fn get_metrics(&self) -> String {
        let _lock = self.data.lock().unwrap();
        let mut buffer = vec![];
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}
