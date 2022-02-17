use std::time::Duration;

use selectme::{Random, StaticSelect};
use tokio::time::{self, Sleep};

#[tokio::main]
pub async fn main() {
    let s1 = time::sleep(Duration::from_millis(100));
    let s2 = time::sleep(Duration::from_millis(200));

    let output: StaticSelect<u8, (Sleep, Sleep), Random, Option<u32>> = selectme::inline! {
        static;

        () = s1 => Some(1),
        _ = s2 => Some(2),
        else => None,
    };

    tokio::pin!(output);

    while let Some(output) = output.as_mut().next().await {
        dbg!(output);
    }
}
