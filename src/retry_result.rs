#[derive(Debug)]
pub enum RetryResult<T, E> {
    Success(T),
    Retry(E), /* Propagated only if all retries exhausted*/
    Abort(E),
}

impl<T, E> From<RetryResult<T, E>> for Result<T, E> {
    fn from(r: RetryResult<T, E>) -> Self {
        match r {
            RetryResult::Success(t) => Ok(t),
            RetryResult::Abort(e) | RetryResult::Retry(e) => Err(e),
        }
    }
}