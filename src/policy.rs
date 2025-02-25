use crate::backoff::*;
use crate::executor::Executor;
use crate::retryer::{ClosureRetryer, Retryer};
use crate::{BackoffPolicy, RetryResult};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt::Debug;

pub const DEFAULT_POLICY: RetryPolicy = RetryPolicy {
    limit: RetryLimit::Unlimited,
    base_delay: 1000,
    delay_time: constant_backoff,
};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum RetryLimit {
    Unlimited,
    Limited(usize),
}

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub limit: RetryLimit,
    pub base_delay: u64,
    pub delay_time: fn(&RetryPolicy, usize) -> u64,
}

impl PartialEq for RetryLimit {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (RetryLimit::Unlimited, RetryLimit::Unlimited) => true,
            (RetryLimit::Limited(a), RetryLimit::Limited(b)) => a == b,
            _ => false,
        }
    }
}

impl PartialEq<usize> for RetryLimit {
    fn eq(&self, other: &usize) -> bool {
        match self {
            RetryLimit::Unlimited => false,
            RetryLimit::Limited(a) => a == other,
        }
    }
}

impl PartialOrd<usize> for RetryLimit {
    fn partial_cmp(&self, count: &usize) -> Option<Ordering> {
        match self {
            RetryLimit::Unlimited => Some(Ordering::Less),
            RetryLimit::Limited(lim) => match count.cmp(lim) {
                Ordering::Less => Some(Ordering::Less),
                Ordering::Equal => Some(Ordering::Equal),
                Ordering::Greater => Some(Ordering::Greater),
            },
        }
    }
}
impl PartialEq<RetryLimit> for usize {
    fn eq(&self, other: &RetryLimit) -> bool {
        other.eq(self)
    }
}

impl PartialOrd<RetryLimit> for usize {
    fn partial_cmp(&self, other: &RetryLimit) -> Option<Ordering> {
        other.partial_cmp(self)
    }
}

impl RetryPolicy {
    pub async fn wait(&self, count: usize) {
        let t = (self.delay_time)(self, count);
        let t = std::time::Duration::from_millis(t);
        tokio::time::sleep(t).await;
    }

    pub fn can_retry(&self, count: usize) -> bool {
        count < self.limit
    }

    /// Runs a function against the given policy
    pub async fn call<'a, Func, RetType, ErrType>(
        &'a self,
        executor: Func,
    ) -> Result<RetType, ErrType>
    where
        Func: Executor<RetType, ErrType> + 'a,
    {
        Retryer {
            policy: crate::util::OwnedOrRef::Ref(self), /* Ref here to avoid consuming a policy we may want to use repeatedly */
            count: 0,
            function: Box::new(&executor),
        }.run().await
    }

    /// Runs a function against the given policy
    pub async fn call_closure<'a, RetType: Send + Sync, ErrType: Send + Sync>(
        &'a self,
        f: impl AsyncFn() -> RetryResult<RetType, ErrType> + Send + Sync,
    ) -> Result<RetType, ErrType> {
        ClosureRetryer {
            policy: crate::util::OwnedOrRef::Ref(self), /* Ref here to avoid consuming a policy we may want to use repeatedly */
            count: 0,
            function: f,
        }.run().await
    }

    pub fn builder() -> RetryPolicyBuilder {
        RetryPolicyBuilder::new()
    }
}

#[derive(Default, Debug)]
pub struct RetryPolicyBuilder {
    limit: Option<RetryLimit>,
    base_delay: Option<u64>,
    backoff_policy: Option<BackoffPolicy>,
}

impl RetryPolicyBuilder {
    /// Creates a new RetryPolicyBuilder
    /// All fields are unset by default
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    /// Creates a new RetryPolicyBuilder with default values. Default Values:
    ///
    /// - limit: Unlimited
    /// - base_delay: 1000
    /// - backoff_policy: constant_backoff
    #[inline]
    pub fn new_with_defaults() -> Self {
        Self {
            limit: Some(RetryLimit::Unlimited),
            base_delay: Some(1000),
            backoff_policy: Some(constant_backoff),
        }
    }

    /// Sets the limit for the RetryPolicy.
    /// Limit is the inclusive upper bound on the number of times a retryable function will be attempted
    /// before converting the error to a final result
    #[inline]
    pub fn limit(mut self, limit: RetryLimit) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the base delay for the RetryPolicy.
    /// Base delay is the time (in milliseconds) to wait before retrying a function.
    /// Does not apply to first attempt.
    /// Subsequent attempts will have their delay calculated by the backoff_policy
    #[inline]
    pub fn base_delay(mut self, base_delay: u64) -> Self {
        self.base_delay = Some(base_delay);
        self
    }

    /// Sets the backoff policy for the RetryPolicy.
    /// The backoff policy is a function that takes the RetryPolicy and the current attempt number
    /// and returns the time (in milliseconds) to wait before retrying the function
    /// Is called after the previous attempt has failed
    #[inline]
    pub fn backoff_policy(mut self, backoff_policy: fn(&RetryPolicy, usize) -> u64) -> Self {
        self.backoff_policy = Some(backoff_policy);
        self
    }

    /// Builds a RetryPolicy with the given parameters from the builder
    ///
    /// # Panics
    ///
    /// Panics if any of the required fields are not set. To avoid panics, use
    /// try_build (returns Result<RetryPolicy, RetryPolicyBuilderError>)
    /// or build_with_defaults (returns RetryPolicy with default values for unset fields)
    #[inline]
    pub fn build(self) -> RetryPolicy {
        RetryPolicy {
            limit: self.limit.expect("limit be set before calling build"),
            base_delay: self
                .base_delay
                .expect("base_delay be set before calling build"),
            delay_time: self
                .backoff_policy
                .expect("delay_time be set before calling build"),
        }
    }

    /// Builds a RetryPolicy with the given parameters from the builder.
    /// If any required fields are not set, the default values will be used.
    /// Default Values:
    /// - limit: Unlimited
    /// - base_delay: 1000
    /// - backoff_policy: constant_backoff
    ///
    /// Unlike build, this method will not panic if any required fields are not set
    #[inline]
    pub fn build_with_defaults(self) -> RetryPolicy {
        RetryPolicy {
            limit: self.limit.unwrap_or(RetryLimit::Unlimited),
            base_delay: self.base_delay.unwrap_or(1000),
            delay_time: self.backoff_policy.unwrap_or(constant_backoff),
        }
    }

    /// Builds a RetryPolicy with the given parameters from the builder.
    /// Any missing fields are added to the error returned
    #[inline]
    pub fn try_build(self) -> Result<RetryPolicy, RetryPolicyBuilderError> {
        let mut error = RetryPolicyBuilderError {
            missing_base_delay: false,
            missing_backoff_policy: false,
            missing_limit: false,
        };

        let mut missing_any = false;
        if self.limit.is_none() {
            error.missing_limit = true;
            missing_any = true;
        }
        if self.base_delay.is_none() {
            error.missing_base_delay = true;
            missing_any = true;
        }
        if self.backoff_policy.is_none() {
            error.missing_backoff_policy = true;
            missing_any = true;
        }

        if missing_any {
            return Err(error);
        }

        Ok(RetryPolicy {
            limit: self.limit.unwrap(),
            base_delay: self.base_delay.unwrap(),
            delay_time: self.backoff_policy.unwrap(),
        })
    }
}

/// Error returned when a RetryPolicyBuilder is missing required fields
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RetryPolicyBuilderError {
    missing_limit: bool,
    missing_base_delay: bool,
    missing_backoff_policy: bool,
}

/// Utility trait to make async closures retryable.
/// This trait is implemented for all async closures that return a RetryResult.
///
/// Can be used to retry the closure immediately with a policy or the default policy.
/// See: retry_rs::policy::DEFAULT_POLICY
///
#[allow(async_fn_in_trait)]
pub trait Retryable<T, E> {
    /// Provided by the retry_rs::Retryable trait, re-exported in prelude
    /// Allows a policy to be provided directly to a closure to retry it
    ///
    /// # Example
    ///
    /// ```rust, ignore
    ///        let policy = RetryPolicy::builder().build_with_defaults();
    ///        let res = (|| async {
    ///            match some_async_function().await {
    ///                Ok(_v) => {
    ///                    Success(())
    ///                }
    ///                Err(_e) => {
    ///                    Retry(())
    ///                }
    ///            }
    ///        }).retry(&policy).await;
    /// ```
    /// # Returns
    ///
    /// Result<T,E> compatible with the RetryResult<T,E> returned by the closure
    ///
    async fn retry(&self, policy: &RetryPolicy) -> Result<T, E>;

    /// Provided by the retry_rs::Retryable trait, re-exported in prelude.
    /// Retry the closure with the default policy. See: retry_rs::policy::DEFAULT_POLICY
    ///
    /// # Example
    ///
    /// ```rust, ignore
    ///        let policy = RetryPolicy::builder().build_with_defaults();
    ///        let res = (|| async {
    ///            match some_async_function(execute().await {
    ///                Ok(_v) => {
    ///                    Success(())
    ///                }
    ///                Err(_e) => {
    ///                    Retry(())
    ///                }
    ///            }
    ///        }).retry_with_default_policy().await;
    /// ```
    /// # Returns
    ///
    /// Result<T,E> compatible with the RetryResult<T,E> returned by the closure
    ///
    async fn retry_with_default_policy(&self) -> Result<T, E>;
}

impl<F, T, E> Retryable<T, E> for F
where
    F: AsyncFn() -> RetryResult<T, E> + Send + Sync,
    T: Send + Sync,
    E: Send + Sync,
{
    async fn retry(&self, policy: &RetryPolicy) -> Result<T, E> {
        policy.call_closure(self).await
    }

    async fn retry_with_default_policy(&self) -> Result<T, E> {
        let policy = DEFAULT_POLICY;
        policy.call_closure(self).await
    }
}
