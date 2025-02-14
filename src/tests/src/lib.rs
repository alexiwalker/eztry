mod agent;

mod tests {
    use crate::agent::*;
    use retry_rs::RetryResult::{Retry, Success, Abort};
    use retry_rs::*;
    use retry_rs_macros::*;


    type DemoStructWithAsync = MutableAgent;
    /*the function here should always pass, its to make sure that what I am passing can be passed to async functions*/
    async fn takes_an_agent(agent: DemoStructWithAsync) -> FallibleResult {
        let mut guard = agent.lock().await;
        guard.execute_async().await
    }
    

    fn retry_5_times() -> RetryPolicy {
        RetryPolicyBuilder::new()
            .limit(RetryLimit::Limited(5))
            .backoff_policy(linear_backoff)
            .base_delay(100)
            .build_with_defaults()
    }

    #[retry(retry_5_times)]
    async fn agent_executor(agent: DemoStructWithAsync) -> RetryResult<u32, u32> {
        let mut guard = agent.lock().await;
        let res = guard.execute_async().await;
        match res {
            Ok(val) => Success(val.get().unwrap() as u32),
            Err(val) => Retry(val.get().unwrap() as u32),
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


    #[retry_prepare]
    async fn prepared_executor(agent: DemoStructWithAsync) -> RetryResult<u32, u32> {
        let mut guard = agent.lock().await;
        let res = guard.execute_async().await;
        match res {
            Ok(val) => Success(val.get().unwrap() as u32),
            Err(val) => {
                let v = val.get().unwrap() as u32;
                if v ==0 {
                    Abort(v)
                } else {
                    Retry(v)
                }
                
            },
        }
    }
    
    #[tokio::test]
    async fn prepared_function() {
        let agent = get_async_demo_agent();
        let res = prepared_executor(agent).retry_with_default_policy().await;
        assert!(res.is_ok())
    }


    fn get_async_demo_agent() -> DemoStructWithAsync {
        FallibleAgent::mutable(FallibleBehaviour::AlwaysSucceed)
    }
}
