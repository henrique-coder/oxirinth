use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use parking_lot::Mutex;
use tokio::sync::watch;

use crate::cache::CacheKey;
use crate::error::EngineError;

type SharedOutcome<V> = Option<Result<V, Arc<EngineError>>>;
type SharedSender<V> = Arc<watch::Sender<SharedOutcome<V>>>;

/// Deduplicates concurrent async work sharing the same [`CacheKey`].
///
/// While one leader executes `work`, any concurrent callers for the identical key
/// await the leader's result without triggering duplicate HTTP requests.
pub struct SingleflightGroup<V> {
    inflight: Mutex<HashMap<CacheKey, SharedSender<V>>>,
}

impl<V> Default for SingleflightGroup<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V> SingleflightGroup<V> {
    /// Create an empty singleflight coalescer.
    pub fn new() -> Self {
        Self {
            inflight: Mutex::new(HashMap::new()),
        }
    }
}

impl<V: Clone + Send + Sync + 'static> SingleflightGroup<V> {
    /// Execute `work` as leader, or await an already-running leader for `key`.
    pub async fn run<F, Fut>(&self, key: CacheKey, work: F) -> Result<V, Arc<EngineError>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<V, EngineError>>,
    {
        let existing_rx = {
            let mut map = self.inflight.lock();
            if let Some(tx) = map.get(&key) {
                Some(tx.subscribe())
            } else {
                let (tx, _rx) = watch::channel(None);
                map.insert(key, Arc::new(tx));
                None
            }
        };

        if let Some(mut rx) = existing_rx {
            return await_leader(&mut rx).await;
        }

        let guard = LeaderGuard {
            group: self,
            key,
            completed: false,
        };
        let outcome = work().await.map_err(Arc::new);
        guard.complete(outcome.clone());
        outcome
    }
}

async fn await_leader<V: Clone>(
    rx: &mut watch::Receiver<SharedOutcome<V>>,
) -> Result<V, Arc<EngineError>> {
    loop {
        if let Some(result) = rx.borrow_and_update().clone() {
            return result;
        }
        if rx.changed().await.is_err() {
            return Err(Arc::new(EngineError::Internal(
                "singleflight leader channel dropped unexpectedly".to_owned(),
            )));
        }
    }
}

struct LeaderGuard<'a, V: Clone> {
    group: &'a SingleflightGroup<V>,
    key: CacheKey,
    completed: bool,
}

impl<V: Clone> LeaderGuard<'_, V> {
    fn complete(mut self, outcome: Result<V, Arc<EngineError>>) {
        self.completed = true;
        if let Some(tx) = self.group.inflight.lock().remove(&self.key) {
            let _ = tx.send(Some(outcome));
        }
    }
}

impl<V: Clone> Drop for LeaderGuard<'_, V> {
    fn drop(&mut self) {
        if !self.completed
            && let Some(tx) = self.group.inflight.lock().remove(&self.key)
        {
            let _ = tx.send(Some(Err(Arc::new(EngineError::Internal(
                "in-flight singleflight leader task was cancelled".to_owned(),
            )))));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use bytes::Bytes;

    use super::*;

    #[tokio::test]
    async fn coalesces_fifty_concurrent_callers_into_one_execution() {
        let group = Arc::new(SingleflightGroup::<Bytes>::new());
        let key =
            CacheKey::from_request("GET", "https://api.modrinth.com/v3/project/sodium", "anon");
        let calls = Arc::new(AtomicUsize::new(0));

        let mut handles = Vec::with_capacity(50);
        for _ in 0..50 {
            let group = Arc::clone(&group);
            let calls = Arc::clone(&calls);
            handles.push(tokio::spawn(async move {
                group
                    .run(key, || async {
                        calls.fetch_add(1, Ordering::SeqCst);
                        tokio::time::sleep(Duration::from_millis(25)).await;
                        Ok(Bytes::from_static(br#"{"id":"AANobbMI"}"#))
                    })
                    .await
            }));
        }

        for handle in handles {
            let res = handle.await.unwrap().unwrap();
            assert_eq!(res.as_ref(), br#"{"id":"AANobbMI"}"#);
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        // A sequential call after completion triggers a fresh flight.
        let second = group
            .run(key, || async {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(Bytes::from_static(br#"{"id":"AANobbMI","v":2}"#))
            })
            .await
            .unwrap();
        assert_eq!(second.as_ref(), br#"{"id":"AANobbMI","v":2}"#);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn notifies_followers_if_leader_task_is_cancelled() {
        let group = Arc::new(SingleflightGroup::<Bytes>::new());
        let key =
            CacheKey::from_request("GET", "https://api.modrinth.com/v3/project/cancel", "anon");

        let leader_group = Arc::clone(&group);
        let leader = tokio::spawn(async move {
            leader_group
                .run(key, || async {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    Ok(Bytes::new())
                })
                .await
        });

        tokio::time::sleep(Duration::from_millis(10)).await;

        let follower_group = Arc::clone(&group);
        let follower = tokio::spawn(async move {
            follower_group
                .run(key, || async { Ok(Bytes::from_static(b"should-not-run")) })
                .await
        });

        tokio::time::sleep(Duration::from_millis(10)).await;
        leader.abort();

        let follower_res = follower.await.unwrap();
        assert!(matches!(
            follower_res.unwrap_err().as_ref(),
            EngineError::Internal(_)
        ));
    }
}
