use crate::executor::Executor;
use crate::retryer::{BoxRetryer, ClosureRetryer};
use crate::{BackoffPolicy, RetryResult};
use std::cmp::Ordering;
use std::fmt::Debug;

pub const DEFAULT_POLICY: RetryPolicy = RetryPolicy {
    limit: RetryLimit::Unlimited,
    base_delay: 1000,
    delay_time: default_next_delay,
};

#[derive(Default, Debug)]
pub enum RetryLimit {
    #[default]
    Unlimited,
    Limited(usize),
}

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

//allow somecount > retrylimit comparison without having to match on the enum
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
        BoxRetryer {
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

pub fn default_next_delay(policy: &RetryPolicy, _count: usize) -> u64 {
    policy.base_delay
}

impl Default for RetryPolicy {
    fn default() -> Self {
        RetryPolicy {
            limit: Default::default(),
            base_delay: 1000,
            delay_time: default_next_delay,
        }
    }
}

#[derive(Default, Debug)]
pub struct RetryPolicyBuilder {
    limit: Option<RetryLimit>,
    base_delay: Option<u64>,
    backoff_policy: Option<BackoffPolicy>,
}

impl RetryPolicyBuilder {
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    #[inline]
    pub fn new_with_defaults() -> Self {
        Self {
            limit: Some(RetryLimit::Unlimited),
            base_delay: Some(1000),
            backoff_policy: Some(default_next_delay),
        }
    }

    #[inline]
    pub fn limit(mut self, limit: RetryLimit) -> Self {
        self.limit = Some(limit);
        self
    }

    #[inline]
    pub fn base_delay(mut self, base_delay: u64) -> Self {
        self.base_delay = Some(base_delay);
        self
    }

    #[inline]
    pub fn backoff_policy(mut self, backoff_policy: fn(&RetryPolicy, usize) -> u64) -> Self {
        self.backoff_policy = Some(backoff_policy);
        self
    }

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
    #[inline]
    pub fn build_with_defaults(self) -> RetryPolicy {
        RetryPolicy {
            limit: self.limit.unwrap_or(RetryLimit::Unlimited),
            base_delay: self.base_delay.unwrap_or(1000),
            delay_time: self.backoff_policy.unwrap_or(default_next_delay),
        }
    }
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

pub struct RetryPolicyBuilderError {
    missing_limit: bool,
    missing_base_delay: bool,
    missing_backoff_policy: bool,
}


 

pub trait Retryable<RetType, ErrType> {
    async fn retry(
        &self,
        policy: &RetryPolicy,
    ) -> Result<RetType, ErrType>;
}

impl<F, T,E> Retryable<T,E> for F where F: AsyncFn() -> RetryResult<T, E> + Send + Sync, T:Send+Sync,E:Send+Sync{
    async fn retry(
        &self,
        policy: &RetryPolicy,
    ) -> Result<T,E> {
        policy.call_closure(self).await
    }
}