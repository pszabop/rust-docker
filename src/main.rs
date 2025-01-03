use redis::AsyncCommands;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    // Connect to Redis
    let client = redis::Client::open("redis://garnetlocal").unwrap();
    let mut con = client.get_multiplexed_async_connection().await.unwrap();

    // Spawn a new task to periodically check the value of "foo"
    tokio::spawn(async move {
        let mut con2 = client.get_multiplexed_async_connection().await.unwrap();
        loop {
            check_foo(&mut con2).await;
            sleep(Duration::from_secs(1)).await;
        }
    });


    // Increment the key "foo" and set an expiry of 5 seconds
    let _: () = con.incr("foo", 1).await.unwrap();
    let _: () = con.expire("foo", 5).await.unwrap();

    sleep(Duration::from_secs(6)).await;

    // increment the key "foo" and check its value 
    let _: () = con.incr("foo", 1).await.unwrap();
    sleep(Duration::from_secs(4)).await;

}

async fn check_foo(con: &mut redis::aio::MultiplexedConnection) {
    // Read the value of "foo"
    let result: Option<String> = con.get("foo").await.unwrap();

    // Print the value of "foo"
    match result {
        Some(value) => println!("Value of 'foo': {}", value),
        None => println!("Value of 'foo' is null"),
    }
}
