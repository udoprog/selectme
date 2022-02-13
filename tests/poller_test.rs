use std::time::Duration;

use tokio::time;

#[tokio::test]
async fn poller_test() {
    let s1 = time::sleep(Duration::from_secs(1));
    tokio::pin!(s1);
    let mut sleep_done = false;

    let s2 = time::sleep(Duration::from_secs(2));
    tokio::pin!(s2);
    let mut sleep2_done = false;

    loop {
        {
            mod private {
                pub static WAKER: ::selectme::StaticWaker = ::selectme::StaticWaker::new();
                pub static WAKER0: ::selectme::PollerWaker =
                    ::selectme::PollerWaker::new(&WAKER, 0);
                pub static WAKER1: ::selectme::PollerWaker =
                    ::selectme::PollerWaker::new(&WAKER, 1);
            }

            let __fut = (Some(s1.as_mut()), Some(s2.as_mut()));

            private::WAKER.reset(if sleep_done { 0 } else { 1 } | if sleep2_done { 0 } else { 2 });

            let mut output = ::selectme::select(&private::WAKER, __fut, |cx, mut __fut, poller| {
                use ::selectme::macros::{Future, Pin, Poll};

                while let Some(index) = poller.next() {
                    match index {
                        0 => {
                            let __fut0 =
                                unsafe { Pin::map_unchecked_mut(__fut.as_mut(), |f| &mut f.0) };

                            if let Some(__fut) = Option::as_pin_mut(__fut0) {
                                if let Poll::Ready(output) =
                                    poller.poll(cx, &private::WAKER0, |cx| Future::poll(__fut, cx))
                                {
                                    sleep_done = true;
                                    return Poll::Ready(1u32);
                                }
                            }
                        }
                        1 => {
                            let mut __fut1 =
                                unsafe { Pin::map_unchecked_mut(__fut.as_mut(), |f| &mut f.1) };

                            if let Some(__fut) = Option::as_pin_mut(__fut1.as_mut()) {
                                if let Poll::Ready(output) =
                                    poller.poll(cx, &private::WAKER1, |cx| Future::poll(__fut, cx))
                                {
                                    __fut1.set(None);
                                    __fut1.set(Some(s2.as_mut()));
                                    sleep2_done = true;
                                    return Poll::Ready(2u32);
                                }
                            }
                        }
                        n => {
                            panic!("no branch with index `{}`", n);
                        }
                    }
                }

                Poll::Pending
            });

            tokio::pin!(output);

            dbg!(output.next().await);
        }
    }
}
