#![feature(test)]
#![feature(maybe_uninit_uninit_array, maybe_uninit_array_assume_init)]

extern crate test;

use std::mem::MaybeUninit;

use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};

use test::bench::Bencher;
use tokio::sync::oneshot;
use tokio::task;

const COUNT: usize = 32;
const ITERATIONS: usize = 10;

struct Scenario {
    timings: [usize; COUNT],
}

impl Scenario {
    /// Setup the current scenario.
    fn build(&self) -> ([oneshot::Receiver<()>; COUNT], [oneshot::Sender<()>; COUNT]) {
        let mut polls = MaybeUninit::uninit_array::<COUNT>();
        let mut triggers = MaybeUninit::uninit_array::<COUNT>();

        for (p, index) in polls.iter_mut().zip(self.timings.iter()) {
            let (tx, rx) = oneshot::channel::<()>();
            p.write(rx);
            triggers[*index].write(tx);
        }

        unsafe {
            let polls = MaybeUninit::array_assume_init(polls);
            let triggers = MaybeUninit::array_assume_init(triggers);
            (polls, triggers)
        }
    }
}

macro_rules! test {
    ($path:path, $polls:expr $(, $($extra:tt)*)*) => {
        tokio::spawn(async move {
            let [mut f0, mut f1, mut f2, mut f3, mut f4, mut f5, mut f6, mut f7, mut f8, mut f9, mut f10, mut f11, mut f12, mut f13, mut f14, mut f15, mut f16, mut f17, mut f18, mut f19, mut f20, mut f21, mut f22, mut f23, mut f24, mut f25, mut f26, mut f27, mut f28, mut f29, mut f30, mut f31] = $polls;
            let mut done = (false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false);

            for _ in 0..COUNT {
                $path! {
                    $($($extra)*)*

                    _ = &mut f0, if !done.0 => { done.0 = true; }
                    _ = &mut f1, if !done.1 => { done.1 = true; }
                    _ = &mut f2, if !done.2 => { done.2 = true; }
                    _ = &mut f3, if !done.3 => { done.3 = true; }
                    _ = &mut f4, if !done.4 => { done.4 = true; }
                    _ = &mut f5, if !done.5 => { done.5 = true; }
                    _ = &mut f6, if !done.6 => { done.6 = true; }
                    _ = &mut f7, if !done.7 => { done.7 = true; }
                    _ = &mut f8, if !done.8 => { done.8 = true; }
                    _ = &mut f9, if !done.9 => { done.9 = true; }
                    _ = &mut f10, if !done.10 => { done.10 = true; }
                    _ = &mut f11, if !done.11 => { done.11 = true; }
                    _ = &mut f12, if !done.12 => { done.12 = true; }
                    _ = &mut f13, if !done.13 => { done.13 = true; }
                    _ = &mut f14, if !done.14 => { done.14 = true; }
                    _ = &mut f15, if !done.15 => { done.15 = true; }
                    _ = &mut f16, if !done.16 => { done.16 = true; }
                    _ = &mut f17, if !done.17 => { done.17 = true; }
                    _ = &mut f18, if !done.18 => { done.18 = true; }
                    _ = &mut f19, if !done.19 => { done.19 = true; }
                    _ = &mut f20, if !done.20 => { done.20 = true; }
                    _ = &mut f21, if !done.21 => { done.21 = true; }
                    _ = &mut f22, if !done.22 => { done.22 = true; }
                    _ = &mut f23, if !done.23 => { done.23 = true; }
                    _ = &mut f24, if !done.24 => { done.24 = true; }
                    _ = &mut f25, if !done.25 => { done.25 = true; }
                    _ = &mut f26, if !done.26 => { done.26 = true; }
                    _ = &mut f27, if !done.27 => { done.27 = true; }
                    _ = &mut f28, if !done.28 => { done.28 = true; }
                    _ = &mut f29, if !done.29 => { done.29 = true; }
                    _ = &mut f30, if !done.30 => { done.30 = true; }
                    _ = &mut f31, if !done.31 => { done.31 = true; }
                }
            }

            assert!(done.0 && done.1 && done.2 && done.3 && done.4 && done.5 && done.6 && done.7 && done.8 && done.9 && done.10 && done.11 && done.12 && done.13 && done.14 && done.15 && done.16 && done.17 && done.18 && done.19 && done.20 && done.21 && done.22 && done.23 && done.24 && done.25 && done.26 && done.27 && done.28 && done.29 && done.30 && done.31);
        })
    }
}

#[bench]
fn tokio_select(b: &mut Bencher) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .build()
        .expect("failed to construct runtime");

    let scenarios = build_scenarios();

    b.iter(|| {
        runtime.block_on(async {
            for scenario in &scenarios {
                let (polls, triggers) = scenario.build();
                let poller = test!(selectme::select, polls);

                let trigger = tokio::spawn(async move {
                    task::yield_now().await;

                    for t in triggers {
                        task::yield_now().await;
                        let _ = t.send(());
                    }
                });

                let _ = tokio::join!(poller, trigger);
            }
        });
    });
}

#[bench]
fn selectme_select(b: &mut Bencher) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .build()
        .expect("failed to construct runtime");

    let scenarios = build_scenarios();

    b.iter(|| {
        runtime.block_on(async {
            for scenario in &scenarios {
                let (polls, triggers) = scenario.build();
                let poller = test!(selectme::select, polls);

                let trigger = tokio::spawn(async move {
                    task::yield_now().await;

                    for t in triggers {
                        task::yield_now().await;
                        let _ = t.send(());
                    }
                });

                let _ = tokio::join!(poller, trigger);
            }
        });
    });
}

fn build_scenarios() -> Vec<Scenario> {
    let mut scenarios = Vec::new();

    let mut rng = StdRng::seed_from_u64(0x0DDB1A5E5BAD5EEDu64);

    for _ in 0..ITERATIONS {
        let mut source = [(0, 0); COUNT];

        for (n, t) in source.iter_mut().enumerate() {
            t.0 = rng.gen::<u32>();
            t.1 = n;
        }

        source.sort_by_key(|t| t.0);

        let mut timings = [0; COUNT];

        for (t, s) in timings.iter_mut().zip(source.iter()) {
            *t = s.1;
        }

        scenarios.push(Scenario { timings });
    }

    scenarios
}
