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
            use ::selectme::__support::{Future, Pin, Poll};

            mod private {
                pub static WAKER: ::selectme::__support::StaticWaker =
                    ::selectme::__support::StaticWaker::new();
                pub static WAKER0: ::selectme::__support::PollerWaker =
                    ::selectme::__support::PollerWaker::new(&WAKER, 0);
                pub static WAKER1: ::selectme::__support::PollerWaker =
                    ::selectme::__support::PollerWaker::new(&WAKER, 1);
            }

            let mut __fut = (Some(s1.as_mut()), Some(s2.as_mut()));

            let initial = if s1_done { 0 } else { 1 } | if s2_done { 0 } else { 2 };
            let mut select = unsafe { ::selectme::__support::poller(&private::WAKER, initial) };

            loop {
                match select.next().await {
                    0 => {
                        let __fut0 = unsafe { Pin::new_unchecked(&mut __fut.0) };

                        if let Some(__fut) = Option::as_pin_mut(__fut0) {
                            if let Poll::Ready(out) =
                                ::selectme::__support::poll_by_ref(&private::WAKER0, |cx| {
                                    Future::poll(__fut, cx)
                                })
                            {
                                select.clear(0);

                                #[allow(irrefutable_let_patterns)]
                                if let () = out {
                                    break {
                                        s1_done = true;
                                        1
                                    };
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
                                select.clear(1);

                                #[allow(irrefutable_let_patterns)]
                                if let () = out {
                                    break {
                                        s2_done = true;
                                        2
                                    };
                                }
                            }
                        }
                    }
                    ::selectme::__support::DISABLED => {
                        break 4;
                    }
                    n => {
                        panic!("no branch with index `{}`", n);
                    }
                }
            }
        };

        if output == 4 {
            break;
        }

        result += output;
    }

    assert_eq!(result, 3);
}
