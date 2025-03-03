use retry_rs::prelude::*;

use std::io::Write;
use std::time::Duration;
use std::sync::Arc;
use std::fs;

use fs2::FileExt;

use tokio::sync::Mutex;

use log::info;


#[derive(Debug, Clone, PartialEq)]
enum ThreadControl {
    RunRetry,
    RunBackground
}

type Control = Arc<Mutex<ThreadControl>>;


#[retry]
async fn write_to_file(ctr:Control) -> RetryResult<(),()> {
    // loop until we know the background function has created the file handle that we are simulating contestion for
    loop {
        let mg = ctr.lock().await;
        if *mg == ThreadControl::RunRetry {
            break
        }
    }


    info!("fg_thread: Attempting to write to file");

    let mut f = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .append(true)
        .open("./test_file.txt").unwrap();

    match f.try_lock_exclusive() {
        Ok(_) => {
            let write = f.write_all(b"Hello, world!");
            match write {
                Ok(_) => {
                    info!("fg_thread: Wrote to file");
                }
                Err(_) => {
                    info!("fg_thread: Failed to write to file");
                }
            }
            Success(())
        }
        Err(_) => {
            info!("fg_thread: Failed to lock file");
            Retry(())
        }
    }
}

/*

This example should always eventually succeed and exit - after the bg thread finishes 100 loops and releases the file

Used to demonstrate behaviour when a retry is needed to get a resource that is locked by another thread or service
but backs off exponentially while it waits

*/


#[tokio::main]
async fn main() {

    env_logger::builder().filter_level(log::LevelFilter::Info).init();

    let policy = RetryPolicy::builder()
        .limit(RetryLimit::Unlimited)
        .backoff_policy(exponential_backoff)
        .base_delay(50)
        .build();

    retry_rs::global::set_default_policy(policy);

    let ctrl = Arc::new(Mutex::new(ThreadControl::RunBackground));

    let thread = tokio::spawn(bg_thread(ctrl.clone()));


    let _ = write_to_file(ctrl).await;

    let _ = thread.await;

}


/// This simulates a busy FS that may have file operations fail due to file contention
async fn bg_thread(ctrl: Control) -> () {

    { /* ensure the retry thread isnt running while we get the file*/
        let mut mg =ctrl.lock().await;
        *mg = ThreadControl::RunBackground;
    }


    let mut f = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .append(true)
        .open("./test_file.txt")
        .unwrap();


    f.try_lock_exclusive().expect("bg_thread: Failed to lock file");


    { /* allows the retry thread to run and attempt to get the file - which it wont, because we have the file handle above*/
        let mut mg =ctrl.lock().await;
        *mg = ThreadControl::RunRetry;
    }

    info!("bg_thread: Releasing Mutex to allow retry thread to run");


    let mut i = 1;
    loop {
        let wr = f.write_all(format!("{}\n", i).as_bytes());

        match wr {
            Ok(_) => {},
            Err(e) => {
                println!("bg_thread: Error writing to file: {:?}", e);
            }
        }

        tokio::time::sleep(Duration::from_millis(100)).await;

        i += 1;

        if i == 100 {
            break;
        }
    }


    info!("bg_thread: thread with lock exiting - retry should succeed now");

}
