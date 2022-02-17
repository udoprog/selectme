# selectme

[<img alt="github" src="https://img.shields.io/badge/github-udoprog/selectme?style=for-the-badge&logo=github" height="20">](https://github.com/udoprog/selectme)
[<img alt="crates.io" src="https://img.shields.io/crates/v/selectme.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/selectme)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-selectme?style=for-the-badge&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20">](https://docs.rs/selectme)
[<img alt="build status" src="https://img.shields.io/github/workflow/status/udoprog/selectme/CI/main?style=for-the-badge" height="20">](https://github.com/udoprog/selectme/actions?query=branch%3Amain)

A fast and fair select! implementation for asynchronous programming.

See the [select!] or [inline!] macros for documentation.

<br>

### Usage

Add the following to your `Cargo.toml`:

```toml
selectme = "0.4.4"
```

<br>

### Examples

The following is a simple example showcasing two branches being polled
concurrently. For more documentation see [select!].

```rust
async fn do_stuff_async() {
    // work here
}

async fn more_async_work() {
    // work here
}

selectme::select! {
    _ = do_stuff_async() => {
        println!("do_stuff_async() completed first")
    }
    _ = more_async_work() => {
        println!("more_async_work() completed first")
    }
};
```

<br>

## The `inline!` macro

The [inline!] macro provides an *inlined* variant of the [select!] macro.

Instead of awaiting directly it evaluates to an instance of the [Select] or
[StaticSelect] allowing for more efficient multiplexing and complex control
flow.

When combined with the `static;` option it performs the least amount of
magic possible to multiplex multiple asynchronous operations making it
suitable for efficient and custom abstractions.

```rust
use std::time::Duration;
use tokio::time;

async fn async_operation() -> u32 {
    // work here
}

let output = selectme::inline! {
    output = async_operation() => Some(output),
    () = time::sleep(Duration::from_secs(5)) => None,
}.await;

match output {
    Some(output) => {
        assert_eq!(output, 42);
    }
    None => {
        panic!("operation timed out!")
    }
}
```

The more interesting trick is producing a [StaticSelect] through the
`static;` option which can be properly named and used inside of another
future.

```rust
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use pin_project::pin_project;
use selectme::StaticSelect;
use tokio::time::{self, Sleep};

#[pin_project]
struct MyFuture {
    #[pin]
    select: StaticSelect<(Sleep, Sleep), Option<u32>>,
}

impl Future for MyFuture {
    type Output = Option<u32>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        this.select.poll_next(cx)
    }
}

let s1 = time::sleep(Duration::from_millis(100));
let s2 = time::sleep(Duration::from_millis(200));

let my_future = MyFuture {
    select: selectme::inline! {
        static;

        () = s1 => Some(1),
        _ = s2 => Some(2),
        else => None,
    }
};

assert_eq!(my_future.await, Some(1));
```

[select!]: https://docs.rs/selectme/latest/selectme/macro.select.html
[inline!]: https://docs.rs/selectme/latest/selectme/macro.inline.html
[Select]: https://docs.rs/selectme/latest/selectme/struct.Select.html
[StaticSelect]: https://docs.rs/selectme/latest/selectme/struct.StaticSelect.html

License: MIT/Apache-2.0
