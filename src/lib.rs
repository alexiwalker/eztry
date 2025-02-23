#[cfg(feature = "macros")]
extern crate retry_rs_macros;
pub use async_trait::async_trait;
pub use executor::Executor;
pub use policy::RetryPolicy;
pub use retry_result::RetryResult;

#[cfg(feature = "macros")]
pub use retry_rs_macros::*;

mod backoff;
pub mod executor;
pub mod policy;
pub mod retry_result;
pub mod retryer;

pub mod prelude {
    pub use crate::executor::{AsyncFunction, Executor};
    pub use crate::policy::{RetryLimit, RetryPolicy, RetryPolicyBuilder, RetryPolicyBuilderError};
    pub use crate::retry_result::{
        RetryResult, RetryResult::Abort, RetryResult::Retry, RetryResult::Success,
    };

    //automatically add some
    pub use crate::retryer::{ClosureRetryer, Retryer};

    // prelude justification: very useful default methods when making retryable functions
    pub use crate::{abort, retry, success};

    // prelude justification: very useful default methods when building retry policies
    pub use crate::backoff::{constant_backoff, exponential_backoff, linear_backoff};

    // prelude justification: adds a very useful method to async closures
    pub use crate::policy::Retryable;

    #[cfg(feature = "macros")]
    pub use retry_rs_macros::*;

    #[cfg(feature = "macros")]
    pub use crate::async_trait;
}
#[cfg(feature = "macros")]
pub mod macros {
    pub use crate::async_trait;
    pub use retry_rs_macros::*;
}

pub(crate) mod util {
    pub(crate) enum OwnedOrRef<'a, T> {
        Owned(T),
        Ref(&'a T),
    }

    impl<T> OwnedOrRef<'_, T> {
        pub fn as_ref(&self) -> &T {
            match self {
                OwnedOrRef::Owned(p) => p,
                OwnedOrRef::Ref(p) => p,
            }
        }
    }
}

pub type BackoffPolicy = fn(&RetryPolicy, usize) -> u64;

/// Shorthand for RetryResult::Success(value)
#[inline(always)]
pub fn success<T, E>(value: T) -> RetryResult<T, E> {
    RetryResult::Success(value)
}

/// Shorthand for RetryResult::Retry(error)
#[inline(always)]
pub fn retry<T, E>(error: E) -> RetryResult<T, E> {
    RetryResult::Retry(error)
}

/// Shorthand for RetryResult::Abort(error)
#[inline(always)]
pub fn abort<T, E>(error: E) -> RetryResult<T, E> {
    RetryResult::Abort(error)
}
