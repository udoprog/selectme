use std::time::Duration;

use selectme::select;
use tokio::time::sleep;

#[tokio::main]
pub async fn main() {
    let sleep = sleep(Duration::from_secs(2));
    tokio::pin!(sleep);

    select! {
        Some(foo) = sleep => {
            println!("hello");
        }
    }
}
