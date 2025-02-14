mod agent;

mod tests {
    use crate::agent::*;
    use retry_rs::RetryResult::{Retryable, Success};
    use retry_rs::*;
    use retry_rs_macros::retry;

    /*the function here should always pass, its to make sure that what I am passing can be passed to async functions*/
    async fn takes_an_agent(agent: MutableAgent) -> FallibleResult {
        let mut guard = agent.lock().await;
        guard.execute_async().await
    }

    fn retry_5_times() -> RetryPolicy {
        RetryPolicyBuilder::new_with_defaults()
            .limit(RetryLimit::Limited(5))
            .backoff_policy(backoff_policy::linear_backoff)
            .base_delay(100)
            .build_with_defaults()
    }

    #[retry(retry_5_times)]
    async fn agent_executor(agent: MutableAgent) -> RetryResult<u32, u32> {
        let mut guard = agent.lock().await;
        let res = guard.execute_async().await;
        match res {
            Ok(val) => Success(val.get().unwrap() as u32),
            Err(val) => Retryable(val.get().unwrap() as u32),
        }
    }

    #[tokio::test]
    async fn agent_succeeds() {
        let agent = FallibleAgent::mutable(FallibleBehaviour::AlwaysSucceed);
        let res = takes_an_agent(agent).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn agent_fails() {
        let agent = FallibleAgent::mutable(FallibleBehaviour::AlwaysFail);
        let res = agent_executor(agent).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn agent_succeeds_after_5() {
        let agent = FallibleAgent::mutable(FallibleBehaviour::SucceedAfter(5));
        let res = agent_executor(agent.clone()).await;
        let c = agent.count().await;
        assert!(res.is_ok());
        assert_eq!(c, 5);
    }
}
