//! In-flight request deduplication.
//!
//! If 20 concurrent requests ask for the same query (same plugin, same args),
//! only 1 actually runs; the other 19 share its result via a shared future.
//!
//! Pattern: tokio::sync::broadcast channel. First caller becomes the producer,
//! second+ callers subscribe and wait for the broadcast.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

use crate::plugin_bridge::PluginResult;

/// Thread-safe tracker of in-flight requests.
/// Key: dedup key (e.g. "gdelt:query=gaza&limit=50")
/// Value: broadcast sender (subscribers receive the result when it completes)
type ResultSender = broadcast::Sender<Result<PluginResult, String>>;

#[derive(Clone, Default)]
pub struct InflightRequests {
    pending: Arc<Mutex<HashMap<String, ResultSender>>>,
}

impl InflightRequests {
    pub fn new() -> Self {
        Self::default()
    }

    /// Run `f` if no in-flight request exists for `key`. Otherwise subscribe to
    /// the in-flight request's result.
    ///
    /// Returns (result, was_executed_by_us) — was_executed=true means we ran `f`,
    /// false means we piggybacked on someone else's request.
    pub async fn call<F, Fut>(&self, key: String, f: F) -> (Result<PluginResult, String>, bool)
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<PluginResult, String>> + Send,
    {
        // Try to become the producer
        let mut rx = {
            let mut map = self.pending.lock().unwrap();
            match map.get(&key) {
                Some(tx) => {
                    // Join existing request
                    Some(tx.subscribe())
                }
                None => {
                    // Become producer
                    let (tx, _) = broadcast::channel(1);
                    map.insert(key.clone(), tx);
                    None
                }
            }
        };

        match rx.take() {
            Some(mut rx) => {
                // We're a subscriber — wait for producer to finish
                let result = match rx.recv().await {
                    Ok(r) => r,
                    Err(broadcast::error::RecvError::Closed) => {
                        Err("producer dropped without sending".to_string())
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        Err("lagged behind producer".to_string())
                    }
                };
                (result, false)
            }
            None => {
                // We're the producer — run the work
                let result = f().await;

                // Broadcast result (Clone so each subscriber gets one)
                let shared = match &result {
                    Ok(r) => Ok(r.clone()),
                    Err(e) => Err(e.clone()),
                };

                let mut map = self.pending.lock().unwrap();
                if let Some(tx) = map.remove(&key) {
                    let _ = tx.send(shared); // ignore send errors (no subscribers left)
                }

                (result, true)
            }
        }
    }
}

// PluginResult is not Clone by default — implement it manually
impl Clone for PluginResult {
    fn clone(&self) -> Self {
        Self {
            success: self.success,
            data: self.data.clone(),
            error: self.error.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dedup_concurrent() {
        let dedup = InflightRequests::new();
        let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        // Spawn 10 concurrent identical requests
        let mut handles = Vec::new();
        for _ in 0..10 {
            let dedup = dedup.clone();
            let counter = counter.clone();
            handles.push(tokio::spawn(async move {
                dedup
                    .call("test".to_string(), || {
                        let counter = counter.clone();
                        async move {
                            // Simulate slow plugin call
                            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                            counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            Ok(PluginResult {
                                success: true,
                                data: Some(serde_json::json!({"result": 42})),
                                error: None,
                            })
                        }
                    })
                    .await
            }));
        }

        let results: Vec<_> = futures::future::join_all(handles).await;

        // Exactly one execution
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);

        // All 10 got success
        let success_count = results
            .iter()
            .filter(|r| matches!(r, Ok((Ok(res), _)) if res.success))
            .count();
        assert_eq!(success_count, 10);
    }
}
