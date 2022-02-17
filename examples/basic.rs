use std::time::Duration;

use tokio::time;

#[selectme::main]
pub(crate) async fn main() {
    let s1 = time::sleep(Duration::from_millis(100));
    tokio::pin!(s1);
    let mut s1_done = false;

    let s2 = time::sleep(Duration::from_millis(200));
    tokio::pin!(s2);
    let mut s2_done = false;

    loop {
        let output = selectme::select! {
            () = &mut s1, if !s1_done => {
                s1_done = true;
                Some(1)
            }
            _ = &mut s2, if !s2_done => {
                s2_done = true;
                Some(2)
            }
            else => {
                None
            }
        };

        let output = match output {
            Some(output) => output,
            None => break,
        };

        dbg!(output);
    }
}
