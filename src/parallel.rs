//! Parallel execution utilities with rate-limit-aware retry.
//!
//! Provides a concurrency-limited parallel executor for bulk `OneLake` operations
//! that respects Fabric API rate limits (429/503) via exponential backoff with jitter.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio::time::sleep;

use crate::errors::ErrorCode;

/// Default maximum concurrency for parallel I/O operations.
/// Computed as `min(available_parallelism * 4, 16)` at runtime.
#[inline]
pub fn default_concurrency() -> usize {
    let cpus = std::thread::available_parallelism().map_or(4, std::num::NonZero::get);
    cpus.saturating_mul(4).clamp(2, 16)
}

/// Maximum number of retry attempts for transient/rate-limited errors.
const MAX_RETRIES: u32 = 5;

/// Result of a single parallel operation.
#[derive(Debug, Clone)]
pub struct OpResult<T> {
    pub index: usize,
    pub result: Result<T, OpError>,
}

/// An error from a parallel operation, preserving the item context.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct OpError {
    pub message: String,
    pub code: ErrorCode,
    pub retries_exhausted: bool,
}

impl std::fmt::Display for OpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

/// Execute a batch of operations in parallel with bounded concurrency and retry.
///
/// - `items`: The work items to process
/// - `concurrency`: Maximum in-flight operations
/// - `op`: Async closure that processes each item; returns `Result<T, anyhow::Error>`
///
/// All items are processed (collect-all, no fail-fast). Returns results in input order.
#[allow(clippy::future_not_send)]
pub async fn execute_parallel<T, I, F, Fut>(
    items: Vec<I>,
    concurrency: usize,
    op: F,
) -> Vec<OpResult<T>>
where
    I: Clone + Send + Sync + 'static,
    T: Send + 'static,
    F: Fn(I) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<T, anyhow::Error>> + Send + 'static,
{
    let op = Arc::new(op);
    let semaphore = Arc::new(Semaphore::new(concurrency));
    let mut join_set = JoinSet::new();

    for (index, item) in items.into_iter().enumerate() {
        let op = op.clone();
        let sem = semaphore.clone();
        join_set.spawn(async move {
            let _permit = sem.acquire().await.unwrap_or_else(|_| unreachable!());
            let result = retry_with_backoff(|| op(item.clone())).await;
            OpResult { index, result }
        });
    }

    let mut results = Vec::new();
    while let Some(res) = join_set.join_next().await {
        if let Ok(op_result) = res {
            results.push(op_result);
        }
    }
    results.sort_by_key(|r| r.index);
    results
}

/// Retry an async operation with exponential backoff on transient/rate-limited errors.
async fn retry_with_backoff<T, F, Fut>(op: F) -> Result<T, OpError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, anyhow::Error>>,
{
    let mut attempt = 0;
    loop {
        match op().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                let is_retryable = is_retryable_error(&e);
                attempt += 1;

                if !is_retryable || attempt > MAX_RETRIES {
                    let code = extract_error_code(&e);
                    return Err(OpError {
                        message: e.to_string(),
                        code,
                        retries_exhausted: attempt > MAX_RETRIES,
                    });
                }

                let backoff = compute_backoff(attempt);
                eprintln!(
                    "  \u{27f3} retry {attempt}/{MAX_RETRIES} after {:.1}s: {}",
                    backoff.as_secs_f64(),
                    e
                );
                sleep(backoff).await;
            }
        }
    }
}

/// Determine if an error is transient and worth retrying.
fn is_retryable_error(err: &anyhow::Error) -> bool {
    err.downcast_ref::<crate::errors::FabioError>().map_or_else(
        || {
            // Network/IO errors from reqwest are retryable
            err.downcast_ref::<reqwest::Error>()
                .is_some_and(|re| re.is_timeout() || re.is_connect() || re.is_request())
        },
        |fabio_err| {
            matches!(
                fabio_err.code,
                ErrorCode::RateLimited | ErrorCode::NetworkError | ErrorCode::ApiError
            )
        },
    )
}

/// Extract an `ErrorCode` from an anyhow error chain.
fn extract_error_code(err: &anyhow::Error) -> ErrorCode {
    err.downcast_ref::<crate::errors::FabioError>()
        .map_or(ErrorCode::Unknown, |e| e.code)
}

/// Compute backoff with jitter for a given attempt number.
fn compute_backoff(attempt: u32) -> Duration {
    let base_ms: u64 = 500; // BASE_BACKOFF in ms
    let exp_ms = base_ms.saturating_mul(1 << attempt.min(6));
    let max_ms: u64 = 30_000; // MAX_BACKOFF in ms
    let capped_ms = exp_ms.min(max_ms);
    // Simple jitter: 75%-125% of computed delay
    let jitter_pct = fastrand_u64() % 50; // 0..49
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let final_ms = capped_ms * (75 + jitter_pct) / 100;
    Duration::from_millis(final_ms)
}

/// Fast pseudo-random u64 using thread-local state (no external crate needed).
fn fastrand_u64() -> u64 {
    use std::cell::Cell;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    thread_local! {
        static STATE: Cell<u64> = Cell::new({
            let mut hasher = DefaultHasher::new();
            std::thread::current().id().hash(&mut hasher);
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
                .hash(&mut hasher);
            hasher.finish()
        });
    }

    STATE.with(|s| {
        // xorshift64
        let mut x = s.get();
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        s.set(x);
        x
    })
}

/// Summary of a parallel batch execution.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BatchSummary {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub failures: Vec<FailureDetail>,
}

/// Detail about a single failed operation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FailureDetail {
    pub item: String,
    pub error: String,
    pub code: String,
}

impl BatchSummary {
    /// Build a summary from parallel operation results.
    pub fn from_results<T>(results: &[OpResult<T>], item_names: &[String]) -> Self {
        let total = results.len();
        let mut succeeded = 0;
        let mut failures = Vec::new();

        for result in results {
            match &result.result {
                Ok(_) => succeeded += 1,
                Err(e) => {
                    let item_name = item_names
                        .get(result.index)
                        .cloned()
                        .unwrap_or_else(|| format!("item[{}]", result.index));
                    failures.push(FailureDetail {
                        item: item_name,
                        error: e.message.clone(),
                        code: e.code.to_string(),
                    });
                }
            }
        }

        Self {
            total,
            succeeded,
            failed: failures.len(),
            failures,
        }
    }

    /// Returns true if all operations succeeded.
    #[inline]
    pub const fn all_succeeded(&self) -> bool {
        self.failed == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_concurrency_is_reasonable() {
        let c = default_concurrency();
        assert!(c >= 2, "concurrency should be at least 2, got {c}");
        assert!(c <= 16, "concurrency should be at most 16, got {c}");
    }

    #[test]
    fn compute_backoff_increases_with_attempts() {
        let b1 = compute_backoff(1);
        let b3 = compute_backoff(3);
        // b3 should generally be larger (8x base vs 2x base), allowing for jitter
        assert!(
            b3.as_millis() > b1.as_millis() / 2,
            "backoff should generally increase"
        );
    }

    #[test]
    fn compute_backoff_capped_at_max() {
        let b = compute_backoff(20);
        // Max is 30_000ms * (75+49)/100 = 30_000 * 124/100 = 37_200ms
        assert!(
            b <= Duration::from_millis(37_500),
            "backoff should be capped near 30s with jitter, got {b:?}",
        );
    }

    #[tokio::test]
    async fn execute_parallel_processes_all_items() {
        let items: Vec<u32> = (0..10).collect();
        let results = execute_parallel(items, 4, |i| async move { Ok(i * 2) }).await;
        assert_eq!(results.len(), 10);
        let successes: usize = results.iter().filter(|r| r.result.is_ok()).count();
        assert_eq!(successes, 10);
    }

    #[tokio::test]
    async fn execute_parallel_collects_errors_without_fail_fast() {
        let items: Vec<u32> = (0..5).collect();
        let results = execute_parallel(items, 4, |i| async move {
            if i == 2 {
                Err(crate::errors::FabioError::not_found("item 2 missing").into())
            } else {
                Ok(i)
            }
        })
        .await;

        assert_eq!(results.len(), 5);
        let failures: usize = results.iter().filter(|r| r.result.is_err()).count();
        // NOT_FOUND is not retryable, so it fails immediately
        assert_eq!(failures, 1);
    }

    #[test]
    fn batch_summary_reports_correctly() {
        let results = vec![
            OpResult {
                index: 0,
                result: Ok(()),
            },
            OpResult {
                index: 1,
                result: Err(OpError {
                    message: "rate limited".to_string(),
                    code: ErrorCode::RateLimited,
                    retries_exhausted: true,
                }),
            },
            OpResult {
                index: 2,
                result: Ok(()),
            },
        ];
        let names = vec![
            "file_a.parquet".to_string(),
            "file_b.parquet".to_string(),
            "file_c.parquet".to_string(),
        ];
        let summary = BatchSummary::from_results(&results, &names);
        assert_eq!(summary.total, 3);
        assert_eq!(summary.succeeded, 2);
        assert_eq!(summary.failed, 1);
        assert!(!summary.all_succeeded());
        assert_eq!(summary.failures[0].item, "file_b.parquet");
    }

    #[test]
    fn fastrand_produces_varying_values() {
        let a = fastrand_u64();
        let b = fastrand_u64();
        // Extremely unlikely to be equal
        assert_ne!(a, b);
    }

    #[tokio::test]
    async fn execute_parallel_retries_transient_errors() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let call_count = std::sync::Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let items: Vec<u32> = vec![1];
        let results = execute_parallel(items, 1, move |_i| {
            let count = call_count_clone.clone();
            async move {
                let attempt = count.fetch_add(1, Ordering::SeqCst);
                if attempt < 2 {
                    // First two attempts fail with a retryable error
                    Err(
                        crate::errors::FabioError::new(ErrorCode::RateLimited, "rate limited")
                            .into(),
                    )
                } else {
                    Ok(42)
                }
            }
        })
        .await;

        assert_eq!(results.len(), 1);
        assert!(results[0].result.is_ok());
        // Should have been called 3 times (2 failures + 1 success)
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn execute_parallel_respects_concurrency_limit() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use tokio::time::Duration;

        let max_concurrent = std::sync::Arc::new(AtomicUsize::new(0));
        let current = std::sync::Arc::new(AtomicUsize::new(0));

        let items: Vec<u32> = (0..10).collect();
        let max_clone = max_concurrent.clone();
        let cur_clone = current.clone();

        let _results = execute_parallel(items, 3, move |i| {
            let max_c = max_clone.clone();
            let cur_c = cur_clone.clone();
            async move {
                let c = cur_c.fetch_add(1, Ordering::SeqCst) + 1;
                max_c.fetch_max(c, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(10)).await;
                cur_c.fetch_sub(1, Ordering::SeqCst);
                Ok(i)
            }
        })
        .await;

        let observed_max = max_concurrent.load(Ordering::SeqCst);
        assert!(
            observed_max <= 3,
            "observed max concurrency {observed_max} should be <= 3"
        );
    }
}
