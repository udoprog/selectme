use std::time::Duration;

use selectme::select;
use tokio::time;

#[tokio::main]
pub async fn main() {
    let s1 = time::sleep(Duration::from_secs(2));
    tokio::pin!(s1);

    let mut s1_done = false;

    let mut s2 = time::interval(Duration::from_secs(5));

    loop {
        let output = select! {
            () = &mut s1 if !s1_done => {
                s1_done = true;
                None
            }
            mut instant = s2.tick() => {
                instant = tokio::time::Instant::now();
                Some(instant)
            }
        };

        dbg!(output);
    }
}
