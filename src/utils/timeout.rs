use futures::{pin_mut, select, Future, FutureExt};
use gloo_timers::future::TimeoutFuture;
use thiserror::Error;

pub fn timeout(duration: std::time::Duration) -> TimeoutFuture {
    TimeoutFuture::new(duration.as_millis() as u32)
}

#[derive(Debug, Error)]
pub enum EnforceTimeoutError {
    #[error("deadline has elapsed")]
    Elapsed,
}

pub async fn enforce_timeout<F>(
    deadline: std::time::Duration,
    f: F,
) -> Result<F::Output, EnforceTimeoutError>
where
    F: Future,
{
    let mut timeout = timeout(deadline).fuse();
    let f = f.fuse();
    pin_mut!(f);

    select! {
        a_res = f => Ok(a_res),
        _ = timeout => Err(EnforceTimeoutError::Elapsed),
    }
}
