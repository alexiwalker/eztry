use crate::policy::RetryPolicy;
use crate::prelude::AsyncFunction;
use crate::retry_result::RetryResult;
use crate::{util, Executor};
use std::fmt::Debug;


pub struct BoxRetryer<'a, T, E> {
    pub(crate) policy: util::OwnedOrRef<'a, RetryPolicy>,
    pub(crate) count: usize, /* not pub, meant to be internal only */
    pub(crate) function: AsyncFunction<'a, T, E>,
}

impl<T, E> BoxRetryer<'_, T, E> {
    pub async fn run(mut self) -> Result<T, E> {
        let f = &self.function;
        let policy = self.policy.as_ref();
        self.count = 0;
        loop {
            self.count += 1;
            let r = f.execute().await;
            match r {
                RetryResult::Success(v) => return Ok(v),
                RetryResult::Abort(v) => return Err(v),
                RetryResult::Retry(e) => {
                    if self.count >= policy.limit {
                        return Err(e);
                    }
                    policy.wait(self.count).await
                }
            }
        }
    }

    pub fn set_policy(&mut self, policy: RetryPolicy) {
        self.policy = util::OwnedOrRef::Owned(policy);
    }
    pub fn count(&self) -> usize {
        self.count
    }
}


pub(crate) struct ClosureRetryer<'a, T, E, F> where F: AsyncFn() -> RetryResult<T, E> + Send + Sync {
    pub(crate) policy: util::OwnedOrRef<'a, RetryPolicy>,
    pub(crate) count: usize, /* not pub, meant to be internal only */
    pub(crate) function: F
}

impl<T, E, F> ClosureRetryer<'_, T, E, F> where F: AsyncFn() -> RetryResult<T, E> + Send + Sync {
    pub async fn run(mut self) -> Result<T, E> {
        let f = &self.function;
        let policy = self.policy.as_ref();
        self.count = 0;
        loop {
            self.count += 1;
            let r = f().await;
            match r {
                RetryResult::Success(v) => return Ok(v),
                RetryResult::Abort(v) => return Err(v),
                RetryResult::Retry(e) => {
                    if self.count >= policy.limit {
                        return Err(e);
                    }
                    policy.wait(self.count).await
                }
            }
        }
    }

    pub fn set_policy(&mut self, policy: RetryPolicy) {
        self.policy = util::OwnedOrRef::Owned(policy);
    }
    pub fn count(&self) -> usize {
        self.count
    }
}