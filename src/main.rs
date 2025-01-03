// the documentation never mentions how to do this. I had to have
// the AI figure it out for me.  This is why I am writing this
// example.  I hope it helps someone else.
use deadpool_redis::{redis::{AsyncCommands}, Config, Runtime};
use tokio::time::{sleep, Duration, Instant};

#[tokio::main]
async fn main() {
    let cfg = Config::from_url("redis://garnetlocal");
    let pool = cfg.create_pool(Some(Runtime::Tokio1)).unwrap();

    // Spawn a new task to periodically check the value of "foo"
    let pool2 = pool.clone();
    tokio::spawn(async move {
        let mut con2 = pool2.get().await.unwrap();
        loop {
            check_foo(&mut con2).await;
            sleep(Duration::from_secs(1)).await;
        }
    });

    let mut con = pool.get().await.unwrap();

    // Increment the key "foo" and set an expiry of 5 seconds
    let _: () = con.incr("foo", 1).await.unwrap();
    let _: () = con.expire("foo", 5).await.unwrap();

    sleep(Duration::from_secs(7)).await;

    // increment the key "foo" and check its value 
    let _: () = con.incr("foo", 1).await.unwrap();
    sleep(Duration::from_secs(4)).await;

}

async fn check_foo(con: &mut redis::aio::MultiplexedConnection) {
    // Start timing
    let start = Instant::now();

    // Read the value of "foo"
    let result: Option<String> = con.get("foo").await.unwrap();

    // Calculate elapsed time
    let duration = start.elapsed();
    let microseconds = duration.as_micros();

    // Print the value of "foo" and the time taken
    match result {
        Some(value) => println!("Value of 'foo': {}, Time taken: {} microseconds", value, microseconds),
        None => println!("Value of 'foo' is null, Time taken: {} microseconds", microseconds),
    }
}
