#![feature(test)]
#![feature(maybe_uninit_uninit_array, maybe_uninit_array_assume_init)]

extern crate test;

use std::mem::MaybeUninit;
use std::thread;

use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};

use test::bench::Bencher;
use tokio::sync::oneshot;

const COUNT: usize = 64;
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
            let [mut v0, mut v1, mut v2, mut v3, mut v4, mut v5, mut v6, mut v7, mut v8, mut v9, mut v10, mut v11, mut v12, mut v13, mut v14, mut v15, mut v16, mut v17, mut v18, mut v19, mut v20, mut v21, mut v22, mut v23, mut v24, mut v25, mut v26, mut v27, mut v28, mut v29, mut v30, mut v31, mut v32, mut v33, mut v34, mut v35, mut v36, mut v37, mut v38, mut v39, mut v40, mut v41, mut v42, mut v43, mut v44, mut v45, mut v46, mut v47, mut v48, mut v49, mut v50, mut v51, mut v52, mut v53, mut v54, mut v55, mut v56, mut v57, mut v58, mut v59, mut v60, mut v61, mut v62, mut v63] = $polls;
            let mut done = (false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false);

            for _ in 0..COUNT {
                $path! {
                    $($($extra)*)*

                    _ = &mut v0, if !done.0 => { done.0 = true; }
                    _ = &mut v1, if !done.1 => { done.1 = true; }
                    _ = &mut v2, if !done.2 => { done.2 = true; }
                    _ = &mut v3, if !done.3 => { done.3 = true; }
                    _ = &mut v4, if !done.4 => { done.4 = true; }
                    _ = &mut v5, if !done.5 => { done.5 = true; }
                    _ = &mut v6, if !done.6 => { done.6 = true; }
                    _ = &mut v7, if !done.7 => { done.7 = true; }
                    _ = &mut v8, if !done.8 => { done.8 = true; }
                    _ = &mut v9, if !done.9 => { done.9 = true; }
                    _ = &mut v10, if !done.10 => { done.10 = true; }
                    _ = &mut v11, if !done.11 => { done.11 = true; }
                    _ = &mut v12, if !done.12 => { done.12 = true; }
                    _ = &mut v13, if !done.13 => { done.13 = true; }
                    _ = &mut v14, if !done.14 => { done.14 = true; }
                    _ = &mut v15, if !done.15 => { done.15 = true; }
                    _ = &mut v16, if !done.16 => { done.16 = true; }
                    _ = &mut v17, if !done.17 => { done.17 = true; }
                    _ = &mut v18, if !done.18 => { done.18 = true; }
                    _ = &mut v19, if !done.19 => { done.19 = true; }
                    _ = &mut v20, if !done.20 => { done.20 = true; }
                    _ = &mut v21, if !done.21 => { done.21 = true; }
                    _ = &mut v22, if !done.22 => { done.22 = true; }
                    _ = &mut v23, if !done.23 => { done.23 = true; }
                    _ = &mut v24, if !done.24 => { done.24 = true; }
                    _ = &mut v25, if !done.25 => { done.25 = true; }
                    _ = &mut v26, if !done.26 => { done.26 = true; }
                    _ = &mut v27, if !done.27 => { done.27 = true; }
                    _ = &mut v28, if !done.28 => { done.28 = true; }
                    _ = &mut v29, if !done.29 => { done.29 = true; }
                    _ = &mut v30, if !done.30 => { done.30 = true; }
                    _ = &mut v31, if !done.31 => { done.31 = true; }
                    _ = &mut v32, if !done.32 => { done.32 = true; }
                    _ = &mut v33, if !done.33 => { done.33 = true; }
                    _ = &mut v34, if !done.34 => { done.34 = true; }
                    _ = &mut v35, if !done.35 => { done.35 = true; }
                    _ = &mut v36, if !done.36 => { done.36 = true; }
                    _ = &mut v37, if !done.37 => { done.37 = true; }
                    _ = &mut v38, if !done.38 => { done.38 = true; }
                    _ = &mut v39, if !done.39 => { done.39 = true; }
                    _ = &mut v40, if !done.40 => { done.40 = true; }
                    _ = &mut v41, if !done.41 => { done.41 = true; }
                    _ = &mut v42, if !done.42 => { done.42 = true; }
                    _ = &mut v43, if !done.43 => { done.43 = true; }
                    _ = &mut v44, if !done.44 => { done.44 = true; }
                    _ = &mut v45, if !done.45 => { done.45 = true; }
                    _ = &mut v46, if !done.46 => { done.46 = true; }
                    _ = &mut v47, if !done.47 => { done.47 = true; }
                    _ = &mut v48, if !done.48 => { done.48 = true; }
                    _ = &mut v49, if !done.49 => { done.49 = true; }
                    _ = &mut v50, if !done.50 => { done.50 = true; }
                    _ = &mut v51, if !done.51 => { done.51 = true; }
                    _ = &mut v52, if !done.52 => { done.52 = true; }
                    _ = &mut v53, if !done.53 => { done.53 = true; }
                    _ = &mut v54, if !done.54 => { done.54 = true; }
                    _ = &mut v55, if !done.55 => { done.55 = true; }
                    _ = &mut v56, if !done.56 => { done.56 = true; }
                    _ = &mut v57, if !done.57 => { done.57 = true; }
                    _ = &mut v58, if !done.58 => { done.58 = true; }
                    _ = &mut v59, if !done.59 => { done.59 = true; }
                    _ = &mut v60, if !done.60 => { done.60 = true; }
                    _ = &mut v61, if !done.61 => { done.61 = true; }
                    _ = &mut v62, if !done.62 => { done.62 = true; }
                    _ = &mut v63, if !done.63 => { done.63 = true; }
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

                let t = thread::spawn(move || {
                    for t in triggers {
                        let _ = t.send(());
                    }
                });

                let _ = poller.await;
                t.join().unwrap();
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

                let t = thread::spawn(move || {
                    for t in triggers {
                        let _ = t.send(());
                    }
                });

                let _ = tokio::join!(poller);
                t.join().unwrap();
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
