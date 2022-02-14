use std::time::Duration;

use tokio::time;

#[tokio::main]
pub async fn main() {
    let s1 = time::sleep(Duration::from_secs(2));
    tokio::pin!(s1);

    let s2 = time::sleep(Duration::from_secs(5));
    tokio::pin!(s2);

    let output = selectme::select! {
        () = s1 => {
            true
        }
        () = s2 => {
            true
        }
        else => {
            false
        }
    };
    tokio::pin!(output);

    loop {
        let output = output.as_mut().next_pinned().await;
        dbg!(output);

        if !output {
            break;
        }
    }
}
