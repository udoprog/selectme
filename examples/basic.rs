use std::time::Duration;

use tokio::time;

#[tokio::main]
pub async fn main() {
    let s1 = time::sleep(Duration::from_secs(2));
    tokio::pin!(s1);

    let mut s1_done = false;

    let mut s2 = time::interval(Duration::from_secs(5));
    let mut s2_done = false;

    loop {
        let output = selectme::inline! {
            () = &mut s1 if !s1_done => {
                s1_done = true;
                None
            }
            mut instant = s2.tick() if !s2_done => {
                s2_done = true;
                instant = tokio::time::Instant::now();
                Some(instant)
            }
            else => {
                None
            }
        };

        dbg!(output);
    }
}
