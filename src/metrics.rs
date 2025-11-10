use std::sync::atomic::{AtomicU64, Ordering};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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