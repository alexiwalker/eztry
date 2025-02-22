use async_trait::async_trait;
use crate::retry_result::RetryResult;
use crate::util;
use crate::policy::{RetryPolicy, DEFAULT_POLICY};
use crate::retryer::{BoxRetryer};

#[async_trait]
pub trait Executor<T, E>: Send + Sync {
    async fn execute(&self) -> RetryResult<T, E>;

    fn default_retry_policy(&self) -> BoxRetryer<T, E>
    where
        Self: Sized
    {

        let __self = self as &dyn Executor<T, E>;
        let b = Box::new(__self);
        BoxRetryer {
            policy: util::OwnedOrRef::Owned(DEFAULT_POLICY),
            count: 0,
            function: b
        }
    }

    async fn retry_with_policy(&self, policy: RetryPolicy) -> Result<T, E>
    where
        Self: Sized + 'static,
        T: Send + Sync,
        E: Send + Sync,
    {
        BoxRetryer {
            policy: util::OwnedOrRef::Owned(policy),
            count: 0,
            function: Box::new(self)
        }
            .run()
            .await
    }

    async fn retry_with_default_policy(&self) -> Result<T, E>
    where
        Self: Sized + 'static,
        T: Send + Sync,
        E: Send + Sync,
    {
        BoxRetryer {
            policy: util::OwnedOrRef::Owned(DEFAULT_POLICY),
            count: 0,
            function: Box::new(self),
        }
            .run()
            .await
    }

    async fn call(self) -> RetryResult<T, E>
    where
        Self: Sized + 'static,
    {
        self.execute().await
    }

    fn use_policy(&self, policy: RetryPolicy) -> BoxRetryer<T, E>
    where
        Self: Sized + 'static,
    {
        BoxRetryer {
            policy: util::OwnedOrRef::Owned(policy),
            count: 0,
            function: Box::new(self),
        }
    }
}


pub type AsyncFunction<'a, T, E> = Box<&'a dyn Executor<T, E>>;
