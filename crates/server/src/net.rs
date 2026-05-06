use std::time::{Duration, Instant};

pub trait Retryable {
    fn should_retry(&self) -> bool;
}

impl Retryable for reqwest::Error {
    fn should_retry(&self) -> bool {
        self.is_connect() || self.is_timeout()
    }
}

pub async fn retry<F, Fut, R, E>(
    factory: F,
    initial_backoff: Duration,
    timeout: Duration,
) -> Result<R, E>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<R, E>>,
    E: Retryable,
{
    let deadline = Instant::now() + timeout;
    let mut backoff = initial_backoff;
    loop {
        match factory().await {
            Ok(res) => return Ok(res),
            Err(err) if err.should_retry() => {
                if deadline < Instant::now() {
                    return Err(err);
                }

                tokio::time::sleep(backoff).await;
                backoff *= 2;
            }
            Err(err) => return Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug)]
    struct RetryableError;
    impl Retryable for RetryableError {
        fn should_retry(&self) -> bool {
            true
        }
    }

    #[derive(Debug)]
    struct FatalError;
    impl Retryable for FatalError {
        fn should_retry(&self) -> bool {
            false
        }
    }

    #[tokio::test]
    async fn retry_succeeds_first_try() {
        let result: Result<i32, RetryableError> = retry(|| async { Ok(42) }, Duration::from_millis(10), Duration::from_secs(1)).await;
        assert_eq!(result.expect("Retry should succeed"), 42);
    }

    #[tokio::test]
    async fn retry_succeeds_after_failures() {
        let count = AtomicUsize::new(0);
        let result: Result<i32, RetryableError> = retry(
            || async {
                let c = count.fetch_add(1, Ordering::SeqCst);
                if c < 2 {
                    Err(RetryableError)
                } else {
                    Ok(42)
                }
            },
            Duration::from_millis(10),
            Duration::from_secs(1),
        )
        .await;
        assert_eq!(result.expect("Retry should succeed"), 42);
        assert_eq!(count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn retry_fails_non_retryable() {
        let result: Result<(), FatalError> = retry(|| async { Err(FatalError) }, Duration::from_millis(10), Duration::from_secs(1)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn retry_fails_after_timeout() {
        let count = AtomicUsize::new(0);
        let result: Result<(), RetryableError> = retry(
            || async {
                count.fetch_add(1, Ordering::SeqCst);
                Err(RetryableError)
            },
            Duration::from_millis(10),
            Duration::from_millis(50),
        )
        .await;
        assert!(result.is_err());
        // Should have tried at least twice (immediate first try + one retry before timeout)
        assert!(count.load(Ordering::SeqCst) >= 2);
    }
}
