use std::time::Duration;

use tokio::time;

#[selectme::main(flavor = "current_thread")]
pub async fn main() {
    let s1 = time::sleep(Duration::from_millis(100));
    let s2 = time::sleep(Duration::from_millis(200));

    let output = selectme::inline! {
        () = s1 => Some(1),
        _ = s2 => Some(2),
        else => None,
    };

    tokio::pin!(output);

    while let Some(output) = output.as_mut().next_pinned().await {
        dbg!(output);
    }
}
