use std::sync::{Arc, Mutex};
use rand::Rng;
use tokio::signal::unix::Signal;
#[derive(Clone, Debug)]

pub(crate) struct FallibleAgent{
    pub behaviour: FallibleBehaviour,
    count:u64
}



#[derive(Clone, Debug)]
pub(crate) enum FallibleBehaviour {
    AlwaysSucceed,
    AlwaysFail,
    FailAfter(u64),
    SucceedAfter(u64),
    RandomisedSuccess(fn(i32) ->bool)
}


// pub(crate) type MutableAgent = Arc<Mutex<FallibleAgent>>;

type TMutex<T> = tokio::sync::Mutex<T>;
type AsyncMutableAgent<T> = Arc<TMutex<T>>;


#[derive(Clone, Debug)]
pub struct MutableAgent(AsyncMutableAgent<FallibleAgent>);

impl MutableAgent {
    pub async fn lock(&self) -> tokio::sync::MutexGuard<'_, FallibleAgent> {
        self.0.lock().await
    }

    pub async fn count(&self) -> u64 {
        self.0.lock().await.count()
    }
}


#[derive(Clone, Debug)]
pub(crate) struct SignalValue(Option<i32>);

impl SignalValue {
    pub fn new() -> Self {
        SignalValue(None)
    }

    pub fn from(value: i32) -> Self {
        SignalValue(Some(value))
    }
    pub fn set(&mut self, value: i32) {
        self.0 = Some(value);
    }
    pub fn get(&self) -> Option<i32> {
        self.0
    }
}

pub(crate) type FallibleResult = Result<SignalValue,SignalValue>;


unsafe impl Send for FallibleAgent {}
unsafe impl Sync for FallibleAgent {}
impl FallibleAgent {

    pub fn count(&self) -> u64 {
        self.count
    }

    pub fn mutable(behaviour: FallibleBehaviour) -> MutableAgent {
        MutableAgent(Arc::new(TMutex::new(FallibleAgent::new(behaviour))))
    }
    pub fn new(behaviour: FallibleBehaviour) -> Self {
        FallibleAgent {
            behaviour,
            count: 0
        }
    }

    pub fn execute(&mut self) -> FallibleResult {
        self.count += 1;
        self.exec()
    }

    pub async fn execute_async(&mut self) -> FallibleResult {
        self.count += 1;
        self.exec()
    }

    fn exec(&mut self) -> FallibleResult {
        match self.behaviour {
            FallibleBehaviour::AlwaysSucceed => Ok(SignalValue::from(self.count as i32)),
            FallibleBehaviour::AlwaysFail => Err(SignalValue::from(self.count as i32)),
            FallibleBehaviour::FailAfter(n) => {
                if self.count < n {
                    Ok(SignalValue::from(self.count as i32))
                } else {
                    Err(SignalValue::from(self.count as i32))
                }
            }
            FallibleBehaviour::SucceedAfter(n) => {
                if self.count < n {
                    Err(SignalValue::from(self.count as i32))
                } else {
                    Ok(SignalValue::from(self.count as i32))
                }
            }
            FallibleBehaviour::RandomisedSuccess(func) => {
                let random_number = generate_random_number();
                if func(random_number) {
                    Ok(SignalValue::from(random_number))
                } else {
                    Err(SignalValue::from(random_number))
                }
            }
        }
    }
}

fn generate_random_number() -> i32 {
    let mut rng = rand::rng();
    rng.random_range(1..=i32::MAX)
}

#[cfg(test)]
mod test {
    use crate::agent::{FallibleAgent, FallibleBehaviour};

    #[test]
    pub fn test_fails_after() {
        use super::*;
        let mut agent = FallibleAgent::new(FallibleBehaviour::FailAfter(2));
        assert_eq!(agent.execute().is_ok(), true);
        assert_eq!(agent.execute().is_ok(), true);
        assert_eq!(agent.execute().is_ok(), false);
    }
    #[test]
    pub fn test_succeeds_after() {
        use super::*;
        let mut agent = FallibleAgent::new(FallibleBehaviour::SucceedAfter(2));
        assert_eq!(agent.execute().is_ok(), false);
        assert_eq!(agent.execute().is_ok(), false);
        assert_eq!(agent.execute().is_ok(), true);
    }

    #[test]
    pub fn test_always_succeeds() {
        use super::*;
        let mut agent = FallibleAgent::new(FallibleBehaviour::AlwaysSucceed);
        for _ in 0..1000 {
            assert_eq!(agent.execute().is_ok(), true);
        }
    }

    #[test]
    pub fn test_always_fails() {
        use super::*;
        let mut agent = FallibleAgent::new(FallibleBehaviour::AlwaysFail);
        for _ in 0..1000 {
            assert_eq!(agent.execute().is_ok(), false);
        }
    }


    #[test]
    pub fn low_chance_of_success(){
        fn low_success_chance(n:i32) -> bool {
            n < 100
        }

        let mut agent = FallibleAgent::new(FallibleBehaviour::RandomisedSuccess(low_success_chance));

        loop {
            let result = agent.execute();
            match result {
                Ok(v) => {
                    assert_eq!(v.get().unwrap() < 100, true);

                    break
                }
                Err(v) => {
                    assert_eq!(v.get().unwrap() >= 100, true);
                }
            }
        }
    }

    #[test]
    pub fn low_chance_of_failure(){
        fn low_success_chance(n:i32) -> bool {
            n > 100
        }

        let mut agent = FallibleAgent::new(FallibleBehaviour::RandomisedSuccess(low_success_chance));

        loop {
            let result = agent.execute();
            match result {
                Ok(v) => {
                    assert_eq!(v.get().unwrap() > 100, true);

                }
                Err(v) => {
                    assert_eq!(v.get().unwrap() <= 100, true);
                    break
                }
            }
        }
    }
}