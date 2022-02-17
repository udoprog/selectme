use std::time::Duration;

use tokio::time;

#[tokio::main]
pub(crate) async fn main() {
    let s1 = time::sleep(Duration::from_millis(100));
    let s2 = time::sleep(Duration::from_millis(200));

    let mut inlined_var = false;

    let output = selectme::inline! {
        () = s1 => {
            inlined_var = true;
            Some(1)
        }
        _ = s2 => Some(2),
        else => None,
    };

    tokio::pin!(output);

    while let Some(output) = output.as_mut().next().await {
        dbg!(output);
    }

    dbg!(inlined_var);
}
