pub use async_trait::async_trait;
pub use executor::Executor;
pub use policy::RetryPolicy;
pub use retry_result::RetryResult;

#[cfg(feature = "macros")]
extern crate retry_rs_macros;

#[cfg(feature = "macros")]
pub use retry_rs_macros::*;

pub mod retry_result;
pub mod policy;
pub mod executor;
pub mod retryer;
mod backoff;

pub mod prelude {
    pub use crate::retry_result::{RetryResult, RetryResult::Abort, RetryResult::Retry, RetryResult::Success};
    pub use crate::{abort, retry, success};
    pub use crate::policy::{RetryLimit, RetryPolicy, RetryPolicyBuilder, RetryPolicyBuilderError};
    pub use crate::executor::{AsyncFunction, Executor};
    pub use crate::backoff::{constant_backoff, exponential_backoff, linear_backoff};
    pub use crate::policy::Retryable;
    #[cfg(feature = "macros")]
    pub use retry_rs_macros::*;

    #[cfg(feature = "macros")]
    pub use crate::async_trait;
}
#[cfg(feature = "macros")]
pub mod macros {
    #[cfg(feature = "macros")]
    pub use retry_rs_macros::*;
    #[cfg(feature = "macros")]
    pub use crate::async_trait;
}


pub(crate) mod util {
    use crate::policy::RetryPolicy;

    pub(crate) enum OwnedOrRef<'a, T> {
    Owned(T),
    Ref(&'a T),
}

    impl OwnedOrRef<'_, RetryPolicy> {
        pub fn as_ref(&self) -> &RetryPolicy {
            match self {
                OwnedOrRef::Owned(p) => p,
                OwnedOrRef::Ref(p) => p,
            }
        }
    }

}


pub type BackoffPolicy = fn(&RetryPolicy, usize) -> u64;

#[inline(always)]
pub fn success<T, E>(v: T) -> RetryResult<T, E> {
    RetryResult::Success(v)
}
#[inline(always)]
pub fn retry<T, E>(e: E) -> RetryResult<T, E> {
    RetryResult::Retry(e)
}
#[inline(always)]
pub fn abort<T, E>(e: E) -> RetryResult<T, E> {
    RetryResult::Abort(e)
}
