#[cfg(test)]
mod agent;
#[cfg(test)]
mod tests {
    use crate::agent::*;
    use retry_rs::prelude::*;

    type DemoStructWithAsync = MutableAgent;
    /*the function here should always pass, its to make sure that what I am passing can be passed to async functions*/
    async fn takes_an_agent(agent: DemoStructWithAsync) -> FallibleResult {
        let mut guard = agent.lock().await;
        guard.execute_async().await
    }

    fn retry_5_times() -> RetryPolicy {
        RetryPolicyBuilder::new()
            .limit(RetryLimit::Limited(5))
            .backoff_policy(backoff::linear_backoff)
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
                if v == 0 {
                    Abort(v)
                } else {
                    Retry(v)
                }
            }
        }
    }

    #[tokio::test]
    async fn prepared_function() {
        let agent = get_async_demo_agent();
        let res = prepared_executor(agent).retry_with_default_policy().await;
        assert!(res.is_ok())
    }


    #[tokio::test]
    async fn runs_only_specified_number() {

        fn policy() -> RetryPolicy {
            RetryPolicyBuilder::new()
                .limit(RetryLimit::Limited(1))
                .backoff_policy(backoff::constant_backoff)
                .base_delay(15)
                .build()
        }

        #[retry(policy)]
        async fn f(agent:MutableAgent) -> RetryResult<(),()> {
            let _ = agent.execute().await;
            Retry(())
        }

        let agent = FallibleAgent::mutable(FallibleBehaviour::AlwaysFail);

        let res = f(agent.clone()).await;

        assert!(res.is_err());
        assert_eq!(agent.count().await , 1);

    }


    #[tokio::test]
    async fn runs_only_specified_number_again() {

        fn policy() -> RetryPolicy {
            RetryPolicyBuilder::new()
                .limit(RetryLimit::Limited(100))
                .backoff_policy(backoff::constant_backoff)
                .base_delay(1)
                .build()
        }

        #[retry(policy)]
        async fn f(agent:MutableAgent) -> RetryResult<(),()> {
            let r = agent.execute().await;
            match r {
                Ok(_) => {
                    Success(())
                }
                Err(_) => {
                    Retry(())
                }
            }
        }

        let agent = FallibleAgent::mutable(FallibleBehaviour::AlwaysFail);

        let res = f(agent.clone()).await;

        assert!(res.is_err());
        assert_eq!(agent.count().await , 100);

    }



    #[tokio::test]
    async fn wont_run_multiple_times_on_success() {

        fn policy() -> RetryPolicy {
            RetryPolicyBuilder::new()
                .limit(RetryLimit::Limited(2))
                .backoff_policy(backoff::constant_backoff)
                .base_delay(15)
                .build()
        }

        #[retry(policy)]
        async fn f(agent:MutableAgent) -> RetryResult<(),()> {
            let r = agent.execute().await;
            match r {
                Ok(_) => {
                    Success(())
                }
                Err(_) => {
                    Retry(())
                }
            }
        }

        let agent = FallibleAgent::mutable(FallibleBehaviour::AlwaysSucceed);

        let res = f(agent.clone()).await;

        assert!(res.is_ok());
        assert_eq!(agent.count().await , 1);

    }

    #[retry_prepare]
    pub async fn ref_function(agent: MutableAgent, some_string:&str) -> RetryResult<(), ()> {
        println!("test string: {}", some_string);
        let r = agent.execute().await;
        match r {
            Ok(_) => {
                Success(())
            }
            Err(_) => {
                Retry(())
            }
        }
    }


    #[tokio::test]
    async fn can_run_prepared_function_on_policy() {

        fn policy() -> RetryPolicy {
            RetryPolicy::builder()
                .limit(RetryLimit::Limited(2))
                .backoff_policy(backoff::constant_backoff)
                .base_delay(15)
                .build()
        }

        #[retry_prepare]
        pub async fn f(agent: MutableAgent) -> RetryResult<(), ()> {
            let r = agent.execute().await;
            match r {
                Ok(_) => {
                    Success(())
                }
                Err(_) => {
                    Retry(())
                }
            }
        }
        #[retry]
        pub async fn f3(agent: MutableAgent) -> RetryResult<(), ()> {
            let r = agent.execute().await;
            match r {
                Ok(_) => {
                    Success(())
                }
                Err(_) => {
                    Retry(())
                }
            }
        }

        let agent = FallibleAgent::mutable(FallibleBehaviour::AlwaysSucceed);
        let v = f(agent.clone());
        let res = policy().call(v).await;
        assert!(res.is_ok());
        assert_eq!(agent.count().await , 1);
        let agent = FallibleAgent::mutable(FallibleBehaviour::AlwaysSucceed);
        let some_str = "hello";
        let v = ref_function(agent.clone(), some_str);
        let res = policy().call(v).await;


        assert!(res.is_ok());
        assert_eq!(agent.count().await , 1);


        let _ = f3(agent.clone()).await;

    }

    fn get_async_demo_agent() -> DemoStructWithAsync {
        FallibleAgent::mutable(FallibleBehaviour::AlwaysSucceed)
    }
}
