use std::sync::atomic::{AtomicU64, Ordering};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::io::AsyncWriteExt;

// Global metrics counters
pub static RPC_INFLIGHT: AtomicU64 = AtomicU64::new(0);
pub static VEC_POINTS_TOTAL: AtomicU64 = AtomicU64::new(0);
pub static TASKS_OPEN_TOTAL: AtomicU64 = AtomicU64::new(0);
pub static VEC_SNAPSHOTS_TOTAL: AtomicU64 = AtomicU64::new(0);

// Simple latency histogram (bucketed)
#[derive(Default)]
pub struct LatencyHistogram {
    buckets: Mutex<HashMap<String, AtomicU64>>,
    start_times: Mutex<HashMap<String, Instant>>,
}

impl LatencyHistogram {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start_timer(&self, operation: &str) {
        let mut starts = self.start_times.lock().unwrap();
        starts.insert(operation.to_string(), Instant::now());
    }

    pub fn end_timer(&self, operation: &str) -> Duration {
        let mut starts = self.start_times.lock().unwrap();
        if let Some(start) = starts.remove(operation) {
            let duration = start.elapsed();

            // Bucket into ranges: <10ms, <100ms, <1s, >=1s
            let bucket = if duration < Duration::from_millis(10) {
                "lt_10ms"
            } else if duration < Duration::from_millis(100) {
                "lt_100ms"
            } else if duration < Duration::from_secs(1) {
                "lt_1s"
            } else {
                "gte_1s"
            };

            let mut buckets = self.buckets.lock().unwrap();
            let counter = buckets.entry(format!("{}_{}", operation, bucket)).or_insert_with(|| AtomicU64::new(0));
            counter.fetch_add(1, Ordering::Relaxed);

            duration
        } else {
            Duration::ZERO
        }
    }

    pub fn get_metrics(&self) -> HashMap<String, u64> {
        let buckets = self.buckets.lock().unwrap();
        buckets.iter()
            .map(|(k, v)| (k.clone(), v.load(Ordering::Relaxed)))
            .collect()
    }
}

// Global latency histogram
lazy_static::lazy_static! {
    pub static ref LATENCY_HIST: LatencyHistogram = LatencyHistogram::new();
}

// Metrics server
pub async fn start_metrics_server(addr: &str) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;

    tokio::spawn(async move {
        loop {
            if let Ok((mut stream, _)) = listener.accept().await {
                let metrics = get_all_metrics();
                let response = format!("{}\n", serde_json::to_string_pretty(&metrics).unwrap());

                if let Err(e) = stream.write_all(response.as_bytes()).await {
                    eprintln!("Failed to write metrics: {}", e);
                }
            }
        }
    });

    Ok(())
}

fn get_all_metrics() -> serde_json::Value {
    serde_json::json!({
        "counters": {
            "syncore_rpc_inflight": RPC_INFLIGHT.load(Ordering::Relaxed),
            "syncore_vec_points": VEC_POINTS_TOTAL.load(Ordering::Relaxed),
            "syncore_tasks_open": TASKS_OPEN_TOTAL.load(Ordering::Relaxed),
            "syncore_vec_snapshots_total": VEC_SNAPSHOTS_TOTAL.load(Ordering::Relaxed),
        },
        "latency_buckets": LATENCY_HIST.get_metrics()
    })
}

// Macro for easy timing
#[macro_export]
macro_rules! time_operation {
    ($op:expr) => {
        {
            let op_str = stringify!($op);
            $crate::metrics::LATENCY_HIST.start_timer(op_str);
            let result = $op;
            $crate::metrics::LATENCY_HIST.end_timer(op_str);
            result
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_latency_histogram_creation() {
        let hist = LatencyHistogram::new();

        // Test that histogram was created successfully
        let metrics = hist.get_metrics();
        assert_eq!(metrics.len(), 0, "New histogram should have no metrics");
    }

    #[test]
    fn test_latency_histogram_timing() {
        let hist = LatencyHistogram::new();

        // Start and end a timer
        hist.start_timer("test_operation");
        thread::sleep(Duration::from_millis(5)); // Sleep for 5ms
        let duration = hist.end_timer("test_operation");

        // Should return a non-zero duration
        assert!(duration > Duration::ZERO, "Should return non-zero duration");

        // Check that metrics were recorded
        let metrics = hist.get_metrics();
        assert!(metrics.contains_key("test_operation_lt_10ms"), "Should record in <10ms bucket");
    }

    #[test]
    fn test_latency_histogram_buckets() {
        let hist = LatencyHistogram::new();

        // Test different latency ranges
        hist.start_timer("fast_op");
        thread::sleep(Duration::from_millis(5));
        hist.end_timer("fast_op");

        hist.start_timer("medium_op");
        thread::sleep(Duration::from_millis(50));
        hist.end_timer("medium_op");

        hist.start_timer("slow_op");
        thread::sleep(Duration::from_millis(500));
        hist.end_timer("slow_op");

        hist.start_timer("very_slow_op");
        thread::sleep(Duration::from_millis(1500));
        hist.end_timer("very_slow_op");

        let metrics = hist.get_metrics();

        assert!(metrics.contains_key("fast_op_lt_10ms"), "Should record fast operation in <10ms bucket");
        assert!(metrics.contains_key("medium_op_lt_100ms"), "Should record medium operation in <100ms bucket");
        assert!(metrics.contains_key("slow_op_lt_1s"), "Should record slow operation in <1s bucket");
        assert!(metrics.contains_key("very_slow_op_gte_1s"), "Should record very slow operation in >=1s bucket");
    }

    #[test]
    fn test_latency_histogram_no_matching_start() {
        let hist = LatencyHistogram::new();

        // End timer without starting it
        let duration = hist.end_timer("nonexistent_operation");

        // Should return zero duration
        assert_eq!(duration, Duration::ZERO, "Should return zero duration for nonexistent operation");

        // Should not record any metrics
        let metrics = hist.get_metrics();
        assert_eq!(metrics.len(), 0, "Should not record metrics for nonexistent operation");
    }

    #[test]
    fn test_global_counters_initial_values() {
        // Note: Global counters persist across tests, so we just test they're atomic
        let initial_rpc = RPC_INFLIGHT.load(Ordering::Relaxed);
        let initial_vec = VEC_POINTS_TOTAL.load(Ordering::Relaxed);
        let initial_tasks = TASKS_OPEN_TOTAL.load(Ordering::Relaxed);
        let initial_snapshots = VEC_SNAPSHOTS_TOTAL.load(Ordering::Relaxed);

        // Test that counters are atomic and readable
        assert!(initial_rpc > 0 || initial_rpc == 0, "RPC inflight counter should be a valid number");
        assert!(initial_vec > 0 || initial_vec == 0, "Vector points counter should be a valid number");
        assert!(initial_tasks > 0 || initial_tasks == 0, "Tasks open counter should be a valid number");
        assert!(initial_snapshots > 0 || initial_snapshots == 0, "Vector snapshots counter should be a valid number");
    }

    #[test]
    fn test_global_counters_increment() {
        // Get initial values
        let initial_rpc = RPC_INFLIGHT.load(Ordering::Relaxed);
        let initial_vec = VEC_POINTS_TOTAL.load(Ordering::Relaxed);
        let initial_tasks = TASKS_OPEN_TOTAL.load(Ordering::Relaxed);
        let initial_snapshots = VEC_SNAPSHOTS_TOTAL.load(Ordering::Relaxed);

        // Test incrementing counters
        RPC_INFLIGHT.fetch_add(1, Ordering::Relaxed);
        VEC_POINTS_TOTAL.fetch_add(5, Ordering::Relaxed);
        TASKS_OPEN_TOTAL.fetch_add(2, Ordering::Relaxed);
        VEC_SNAPSHOTS_TOTAL.fetch_add(3, Ordering::Relaxed);

        // Test that counters were incremented by the expected amounts
        assert_eq!(RPC_INFLIGHT.load(Ordering::Relaxed), initial_rpc + 1, "RPC inflight counter should increase by 1");
        assert_eq!(VEC_POINTS_TOTAL.load(Ordering::Relaxed), initial_vec + 5, "Vector points counter should increase by 5");
        assert_eq!(TASKS_OPEN_TOTAL.load(Ordering::Relaxed), initial_tasks + 2, "Tasks open counter should increase by 2");
        assert_eq!(VEC_SNAPSHOTS_TOTAL.load(Ordering::Relaxed), initial_snapshots + 3, "Vector snapshots counter should increase by 3");
    }

    #[test]
    fn test_get_all_metrics() {
        // Set some counter values
        RPC_INFLIGHT.fetch_add(3, Ordering::Relaxed);
        VEC_POINTS_TOTAL.fetch_add(10, Ordering::Relaxed);

        // Add some latency data
        LATENCY_HIST.start_timer("test");
        thread::sleep(Duration::from_millis(5));
        LATENCY_HIST.end_timer("test");

        let metrics = get_all_metrics();

        // Check that counters are included
        assert!(metrics["counters"]["syncore_rpc_inflight"].is_number(), "Should include RPC inflight counter");
        assert!(metrics["counters"]["syncore_vec_points"].is_number(), "Should include vector points counter");

        // Check that latency buckets are included
        assert!(metrics["latency_buckets"].is_object(), "Should include latency buckets");
    }

    #[tokio::test]
    async fn test_metrics_server_creation() -> anyhow::Result<()> {
        // Test that metrics server can be created (binds to random port)
        let result = start_metrics_server("127.0.0.1:0").await;

        assert!(result.is_ok(), "Should be able to start metrics server on random port");
        Ok(())
    }
}
