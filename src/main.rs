use redis::AsyncCommands;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    // Connect to Redis
    let client = redis::Client::open("redis://garnetlocal").unwrap();
    let mut con = client.get_multiplexed_async_connection().await.unwrap();

    // Increment the key "foo" and set an expiry of 5 seconds
    let _: () = con.incr("foo", 1).await.unwrap();
    let _: () = con.expire("foo", 5).await.unwrap();

    // Create a timer that expires in 2 seconds
    sleep(Duration::from_secs(2)).await;

    // Read the value of "foo"
    let result: Option<String> = con.get("foo").await.unwrap();

    // Check if the value is null
    match result {
        Some(value) => println!("Value of 'foo': {}", value),
        None => println!("Value of 'foo' is null"),
    }

    // Create a timer that expires in 5 seconds (total of 7)
    sleep(Duration::from_secs(5)).await;

    // Read the value of "foo"
    let result: Option<String> = con.get("foo").await.unwrap();

    // Check if the value is null
    match result {
        Some(value) => println!("Value of 'foo': {}", value),
        None => println!("Value of 'foo' is null"),
    }

    // increment the key "foo" and check its value 
    let _: () = con.incr("foo", 1).await.unwrap();
    sleep(Duration::from_secs(1)).await;
    let result: Option<String> = con.get("foo").await.unwrap();
    match result {
        Some(value) => println!("Value of 'foo': {}", value),
        None => println!("Value of 'foo' is null"),
    }

}