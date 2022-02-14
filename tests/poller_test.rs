use std::time::Duration;

use tokio::time;

#[tokio::test]
async fn poller_test() {
    let s1 = time::sleep(Duration::from_millis(100));
    tokio::pin!(s1);
    let mut s1_done = false;

    let s2 = time::sleep(Duration::from_millis(200));
    tokio::pin!(s2);
    let mut s2_done = false;

    let mut result = 0;

    loop {
        let output = {
            mod private {
                pub static WAKER: ::selectme::__support::StaticWaker =
                    ::selectme::__support::StaticWaker::new();
                pub static WAKER0: ::selectme::__support::PollerWaker =
                    ::selectme::__support::PollerWaker::new(&WAKER, 0);
                pub static WAKER1: ::selectme::__support::PollerWaker =
                    ::selectme::__support::PollerWaker::new(&WAKER, 1);
            }

            let __fut = (Some(s1.as_mut()), Some(s2.as_mut()));

            private::WAKER.reset(if s1_done { 0 } else { 1 } | if s2_done { 0 } else { 2 });

            ::selectme::__support::select(&private::WAKER, __fut, |__fut, __mask, __index| {
                use ::selectme::__support::{Future, Pin, Poll};

                match __index {
                    0 => {
                        let __fut0 = unsafe { Pin::new_unchecked(&mut __fut.0) };

                        if let Some(__fut) = Option::as_pin_mut(__fut0) {
                            if let Poll::Ready(out) =
                                ::selectme::__support::poll_by_ref(&private::WAKER0, |cx| {
                                    Future::poll(__fut, cx)
                                })
                            {
                                __mask.clear(__index);

                                #[allow(irrefutable_let_patterns)]
                                if let () = out {
                                    return Poll::Ready({
                                        s1_done = true;
                                        1
                                    });
                                }
                            }
                        }
                    }
                    1 => {
                        let mut __fut1 = unsafe { Pin::new_unchecked(&mut __fut.1) };

                        if let Some(__fut) = Option::as_pin_mut(__fut1.as_mut()) {
                            if let Poll::Ready(out) =
                                ::selectme::__support::poll_by_ref(&private::WAKER1, |cx| {
                                    Future::poll(__fut, cx)
                                })
                            {
                                __fut1.set(None);
                                __mask.clear(__index);

                                #[allow(irrefutable_let_patterns)]
                                if let () = out {
                                    return Poll::Ready({
                                        s2_done = true;
                                        2
                                    });
                                }
                            }
                        }
                    }
                    ::selectme::__support::DISABLED => {
                        return Poll::Ready(4);
                    }
                    n => {
                        panic!("no branch with index `{}`", n);
                    }
                }

                Poll::Pending
            })
        };

        let output = output.await;

        if output == 4 {
            break;
        }

        result += output;
    }

    assert_eq!(result, 3);
}
