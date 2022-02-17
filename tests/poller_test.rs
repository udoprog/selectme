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

            let mut __fut = (Some(s1.as_mut()), Some(s2.as_mut()));

            let initial: u32 = if !s1_done { 1 } else { 0 } + if !s2_done { 2 } else { 0 };

            ::selectme::__support::select(
                initial,
                ::selectme::__support::unbiased(),
                __fut,
                |cx, state, mask, index| {
                    match index {
                        0 => {
                            let __fut0 = unsafe { Pin::map_unchecked_mut(state, |f| &mut f.0) };

                            if let Some(__fut) = Option::as_pin_mut(__fut0) {
                                if let Poll::Ready(out) = Future::poll(__fut, cx) {
                                    mask.clear(0);

                                    #[allow(irrefutable_let_patterns)]
                                    if let () = out {
                                        s1_done = true;
                                        return Poll::Ready(1);
                                    }
                                }
                            }
                        }
                        1 => {
                            let mut __fut1 = unsafe { Pin::map_unchecked_mut(state, |f| &mut f.1) };

                            if let Some(__fut) = Option::as_pin_mut(__fut1.as_mut()) {
                                if let Poll::Ready(out) = Future::poll(__fut, cx) {
                                    __fut1.set(None);
                                    mask.clear(1);

                                    #[allow(irrefutable_let_patterns)]
                                    if let () = out {
                                        s2_done = true;
                                        return Poll::Ready(2);
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
                },
            )
            .await
        };

        if output == 4 {
            break;
        }

        result += output;
    }

    assert_eq!(result, 3);
}
