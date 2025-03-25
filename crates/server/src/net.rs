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
