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

pub mod global {
    use crate::backoff::constant_backoff;
    use crate::policy::RetryLimit;
    use crate::util::StaticWall;
    use crate::{policy, RetryPolicy};
    use std::ops::Deref;
    use std::sync::Mutex;

    const CONST_POLICY: RetryPolicy = RetryPolicy {
        limit: RetryLimit::Unlimited,
        base_delay: 1000,
        delay_time: constant_backoff,
    };
    static DEFAULT_POLICY: Mutex<StaticWall<RetryPolicy>> = Mutex::new(StaticWall(&CONST_POLICY));

    /// Sets the default policy for all retryable functions
    ///
    /// # Warning
    ///
    /// - This will overwrite the default policy for all retryable functions
    ///
    /// - This will leak the provided policy into the global scope. Calling it more than once will cause a memory leak
    pub fn set_default_policy(policy: policy::RetryPolicy) {
        let mut lock = DEFAULT_POLICY.lock().unwrap();
        *lock = StaticWall::leak(policy);
    }

    /// Reset the default policy back to its original values:
    ///
    /// -  limit: RetryLimit::Unlimited
    /// -  base_delay: 1000
    /// -  delay_time: constant_backoff
    pub fn reset_default_policy() {
        let mut lock = DEFAULT_POLICY.lock().unwrap();
        *lock = StaticWall::leak(CONST_POLICY);
    }

    /// Returns a static reference to the global retry policy
    ///
    /// If the default policy has not been set, this will return a policy with the following defaults:
    ///
    /// 1. limit: RetryLimit::Unlimited
    /// 2. base_delay: 1000
    /// 3. delay_time: constant_backoff
    pub fn get_default_policy() -> &'static RetryPolicy {
        let mx = DEFAULT_POLICY.lock().expect("Failed to lock mutex");
        let wall = mx.deref();
        wall.deref()
    }
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

    pub(crate) struct StaticWall<T: 'static>(pub(crate) &'static T);

    impl<T> StaticWall<T> {
        pub fn deref(&self) -> &'static T {
            self.0
        }

        pub(crate) fn leak(item: T) -> Self {
            StaticWall(Box::leak(Box::new(item)))
        }
    }
}

pub type BackoffPolicy = fn(&RetryPolicy, u64) -> u64;

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
