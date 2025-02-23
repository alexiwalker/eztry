use crate::policy::{RetryPolicy, DEFAULT_POLICY};
use crate::retry_result::RetryResult;
use crate::retryer::Retryer;
use crate::util;
use async_trait::async_trait;

#[async_trait]
pub trait Executor<T, E>: Send + Sync {
    async fn execute(&self) -> RetryResult<T, E>;

    /// Prepare the executor to be retried with the default policy. See retry_rs::policy::DEFAULT_POLICY.
    /// Does not begin the retry process until run() is called on the Retryer. The policy can be updated with set_policy().
    fn prepare(&self) -> Retryer<T, E>
    where
        Self: Sized,
    {
        Retryer {
            policy: util::OwnedOrRef::Owned(DEFAULT_POLICY),
            count: 0,
            function: Box::new(self),
        }
    }

    /// Attempts to execute and retry the executor with a policy.
    async fn retry_with_policy(&self, policy: RetryPolicy) -> Result<T, E>
    where
        Self: Sized + 'static,
        T: Send + Sync,
        E: Send + Sync,
    {
        Retryer {
            policy: util::OwnedOrRef::Owned(policy),
            count: 0,
            function: Box::new(self),
        }
        .run()
        .await
    }

    /// Attempts to execute and retry the executor with a borrowed policy
    fn retry_with_policy_ref<'a>(&'a self, policy: &'a RetryPolicy) -> Retryer<'a, T, E>
    where
        Self: Sized + 'static,
    {
        Retryer {
            policy: util::OwnedOrRef::Ref(policy),
            count: 0,
            function: Box::new(self),
        }
    }

    /// Attempts to execute and retry the executor with the default policy. See retry_rs::policy::DEFAULT_POLICY.
    async fn retry_with_default_policy(&self) -> Result<T, E>
    where
        Self: Sized + 'static,
        T: Send + Sync,
        E: Send + Sync,
    {
        Retryer {
            policy: util::OwnedOrRef::Owned(DEFAULT_POLICY),
            count: 0,
            function: Box::new(self),
        }
        .run()
        .await
    }
}

pub type AsyncFunction<'a, T, E> = Box<&'a dyn Executor<T, E>>;
