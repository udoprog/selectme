#![feature(maybe_uninit_uninit_array, maybe_uninit_array_assume_init)]

use std::mem::MaybeUninit;
use std::thread;
use std::time::Instant;

use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};

use tokio::sync::oneshot;

const COUNT: usize = 63;
const ITERATIONS: usize = 100;

macro_rules! test {
    ($path:path, $polls:expr) => {
        tokio::spawn(async move {
            let [mut v0, mut v1, mut v2, mut v3, mut v4, mut v5, mut v6, mut v7, mut v8, mut v9, mut v10, mut v11, mut v12, mut v13, mut v14, mut v15, mut v16, mut v17, mut v18, mut v19, mut v20, mut v21, mut v22, mut v23, mut v24, mut v25, mut v26, mut v27, mut v28, mut v29, mut v30, mut v31, mut v32, mut v33, mut v34, mut v35, mut v36, mut v37, mut v38, mut v39, mut v40, mut v41, mut v42, mut v43, mut v44, mut v45, mut v46, mut v47, mut v48, mut v49, mut v50, mut v51, mut v52, mut v53, mut v54, mut v55, mut v56, mut v57, mut v58, mut v59, mut v60, mut v61, mut v62] = $polls;
            let mut d = (false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false);

            for _ in 0..COUNT {
                $path! {
                    _ = &mut v0, if !d.0 => { d.0 = true; }
                    _ = &mut v1, if !d.1 => { d.1 = true; }
                    _ = &mut v2, if !d.2 => { d.2 = true; }
                    _ = &mut v3, if !d.3 => { d.3 = true; }
                    _ = &mut v4, if !d.4 => { d.4 = true; }
                    _ = &mut v5, if !d.5 => { d.5 = true; }
                    _ = &mut v6, if !d.6 => { d.6 = true; }
                    _ = &mut v7, if !d.7 => { d.7 = true; }
                    _ = &mut v8, if !d.8 => { d.8 = true; }
                    _ = &mut v9, if !d.9 => { d.9 = true; }
                    _ = &mut v10, if !d.10 => { d.10 = true; }
                    _ = &mut v11, if !d.11 => { d.11 = true; }
                    _ = &mut v12, if !d.12 => { d.12 = true; }
                    _ = &mut v13, if !d.13 => { d.13 = true; }
                    _ = &mut v14, if !d.14 => { d.14 = true; }
                    _ = &mut v15, if !d.15 => { d.15 = true; }
                    _ = &mut v16, if !d.16 => { d.16 = true; }
                    _ = &mut v17, if !d.17 => { d.17 = true; }
                    _ = &mut v18, if !d.18 => { d.18 = true; }
                    _ = &mut v19, if !d.19 => { d.19 = true; }
                    _ = &mut v20, if !d.20 => { d.20 = true; }
                    _ = &mut v21, if !d.21 => { d.21 = true; }
                    _ = &mut v22, if !d.22 => { d.22 = true; }
                    _ = &mut v23, if !d.23 => { d.23 = true; }
                    _ = &mut v24, if !d.24 => { d.24 = true; }
                    _ = &mut v25, if !d.25 => { d.25 = true; }
                    _ = &mut v26, if !d.26 => { d.26 = true; }
                    _ = &mut v27, if !d.27 => { d.27 = true; }
                    _ = &mut v28, if !d.28 => { d.28 = true; }
                    _ = &mut v29, if !d.29 => { d.29 = true; }
                    _ = &mut v30, if !d.30 => { d.30 = true; }
                    _ = &mut v31, if !d.31 => { d.31 = true; }
                    _ = &mut v32, if !d.32 => { d.32 = true; }
                    _ = &mut v33, if !d.33 => { d.33 = true; }
                    _ = &mut v34, if !d.34 => { d.34 = true; }
                    _ = &mut v35, if !d.35 => { d.35 = true; }
                    _ = &mut v36, if !d.36 => { d.36 = true; }
                    _ = &mut v37, if !d.37 => { d.37 = true; }
                    _ = &mut v38, if !d.38 => { d.38 = true; }
                    _ = &mut v39, if !d.39 => { d.39 = true; }
                    _ = &mut v40, if !d.40 => { d.40 = true; }
                    _ = &mut v41, if !d.41 => { d.41 = true; }
                    _ = &mut v42, if !d.42 => { d.42 = true; }
                    _ = &mut v43, if !d.43 => { d.43 = true; }
                    _ = &mut v44, if !d.44 => { d.44 = true; }
                    _ = &mut v45, if !d.45 => { d.45 = true; }
                    _ = &mut v46, if !d.46 => { d.46 = true; }
                    _ = &mut v47, if !d.47 => { d.47 = true; }
                    _ = &mut v48, if !d.48 => { d.48 = true; }
                    _ = &mut v49, if !d.49 => { d.49 = true; }
                    _ = &mut v50, if !d.50 => { d.50 = true; }
                    _ = &mut v51, if !d.51 => { d.51 = true; }
                    _ = &mut v52, if !d.52 => { d.52 = true; }
                    _ = &mut v53, if !d.53 => { d.53 = true; }
                    _ = &mut v54, if !d.54 => { d.54 = true; }
                    _ = &mut v55, if !d.55 => { d.55 = true; }
                    _ = &mut v56, if !d.56 => { d.56 = true; }
                    _ = &mut v57, if !d.57 => { d.57 = true; }
                    _ = &mut v58, if !d.58 => { d.58 = true; }
                    _ = &mut v59, if !d.59 => { d.59 = true; }
                    _ = &mut v60, if !d.60 => { d.60 = true; }
                    _ = &mut v61, if !d.61 => { d.61 = true; }
                    _ = &mut v62, if !d.62 => { d.62 = true; }
                }
            }

            assert!(
                d.0  && d.1  && d.2  && d.3  && d.4  && d.5  && d.6  && d.7  &&
                d.8  && d.9  && d.10 && d.11 && d.12 && d.13 && d.14 && d.15 &&
                d.16 && d.17 && d.18 && d.19 && d.20 && d.21 && d.22 && d.23 &&
                d.24 && d.25 && d.26 && d.27 && d.28 && d.29 && d.30 && d.31 &&
                d.32 && d.33 && d.34 && d.35 && d.36 && d.37 && d.38 && d.39 &&
                d.40 && d.41 && d.42 && d.43 && d.44 && d.45 && d.46 && d.47 &&
                d.48 && d.49 && d.50 && d.51 && d.52 && d.53 && d.54 && d.55 &&
                d.56 && d.57 && d.58 && d.59 && d.60 && d.61 && d.62
            );
        })
    }
}

pub fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .build()
        .expect("failed to construct runtime");

    let scenarios = build_scenarios();

    let start = Instant::now();

    runtime.block_on(async {
        for scenario in &scenarios {
            let (polls, triggers) = scenario.build();
            let poller = test!(tokio::select, polls);

            let t = thread::spawn(move || {
                for t in triggers {
                    let _ = t.send(());
                }
            });

            let _ = tokio::join!(poller);
            t.join().unwrap();
        }
    });

    println!("{:?}", Instant::now().duration_since(start));
}

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
