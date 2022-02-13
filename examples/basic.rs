use std::time::Duration;

use tokio::time;

#[tokio::main]
pub async fn main() {
    let s1 = time::sleep(Duration::from_secs(2));
    tokio::pin!(s1);
    let mut s1_done = false;

    let s2 = time::sleep(Duration::from_secs(5));
    tokio::pin!(s2);
    let mut s2_done = false;

    let output = selectme::select! {
        () = s1 if !s1_done => {
            s1_done = true;
            true
        }
        () = s2 if !s2_done => {
            s2_done = true;
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
