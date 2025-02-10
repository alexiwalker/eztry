use rand::Rng;
use retriers_lib::*;
use retriers_macro::retryable;


type RetryType = Retryable<String,String>;



#[retryable]
async fn retry_func(v:u32, s:String, b:bool, f:f32) -> Retryable<Result<String,u32>,String> {
    let mut rng = generate_random_number();
    println!("RNG: {}", rng);
    if rng == 100 {
        let data_1 = v;
        let data_2 =  s;
        let s = format!("{data_1}_{data_2}_{b}::{f}");
        let _ = tokio::fs::write("./tmp_file.txt", &s).await;
        success(Ok(s))
    } else if rng < 5 {
        abort("simulated error".to_string())
    } else {
        retry()
    }
}


fn generate_random_number() -> u8 {
    let mut rng = rand::rng();
    rng.random_range(1..=100)
}

#[tokio::main]
async fn main() {

    let r = retry_func(1u32, "something".to_string(),true,0.01);

    let x = r.default_retry_policy().run().await;


    // let z = __RETRIERS__INTERNAL::retry_func_inner(1u32, "something".to_string(),true,0.01).await;


    __RETRIERS__INTERNAL::

    dbg!(&z);
    println!("here");

    dbg!(x);
}

