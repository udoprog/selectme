use std::time::Duration;

use tokio::time;

#[tokio::test]
async fn poller_test() {
    let sleep = time::sleep(Duration::from_secs(1));
    tokio::pin!(sleep);

    let mut sleep_done = false;

    let sleep2 = time::sleep(Duration::from_secs(2));
    tokio::pin!(sleep2);

    let mut sleep2_done = false;

    loop {
        mod private {
            pub static WAKER: ::selectme::StaticWaker = ::selectme::StaticWaker::new();
            pub static WAKER0: ::selectme::PollerWaker = ::selectme::PollerWaker::new(&WAKER, 0);
            pub static WAKER1: ::selectme::PollerWaker = ::selectme::PollerWaker::new(&WAKER, 1);
        }

        private::WAKER.reset(if sleep_done { 0 } else { 1 } | if sleep2_done { 0 } else { 2 });

        let output = ::selectme::from_fn::<_, _>(&private::WAKER, |cx, poller| {
            use ::selectme::macros::{Future, Pin, Poll};

            while let Some(index) = poller.next() {
                match index {
                    0 => {
                        if let Poll::Ready(output) = poller.poll(cx, &private::WAKER0, |cx| {
                            Future::poll(Pin::as_mut(&mut sleep), cx)
                        }) {
                            sleep_done = true;
                            return Poll::Ready(1u32);
                        }
                    }
                    1 => {
                        if let Poll::Ready(output) = poller.poll(cx, &private::WAKER1, |cx| {
                            Future::poll(Pin::as_mut(&mut sleep2), cx)
                        }) {
                            sleep2_done = true;
                            return Poll::Ready(2u32);
                        }
                    }
                    n => {
                        panic!("no branch with index `{}`", n);
                    }
                }
            }

            Poll::Pending
        })
        .await;

        dbg!(output);
    }
}
