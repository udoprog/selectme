use core::cell::UnsafeCell;
use core::fmt;
use core::hint;
use core::ops::Deref;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering::{AcqRel, Acquire, Release};
use core::task::Waker;

pub struct AtomicWaker {
    state: AtomicUsize,
    waker: UnsafeCell<Option<Waker>>,
}

const WAITING: usize = 0;
const REGISTERING: usize = 0b01;
const WAKING: usize = 0b10;

impl AtomicWaker {
    pub const fn new() -> Self {
        trait AssertSync: Sync {}
        impl AssertSync for Waker {}

        Self {
            state: AtomicUsize::new(WAITING),
            waker: UnsafeCell::new(None),
        }
    }

    pub fn register(&self, waker: &Waker) {
        let res = self
            .state
            .compare_exchange(WAITING, REGISTERING, Acquire, Acquire)
            .unwrap_or_else(|x| x);

        match res {
            WAITING => {
                unsafe {
                    let guard = UnlockGuard {
                        state: &self.state,
                        waker: &self.waker,
                    };

                    // Note: must not hold this reference across the unlock
                    // below.
                    let waker_mut = &mut *guard.waker.get();

                    // Locked acquired, update the waker cell
                    if let Some(other) = waker_mut {
                        // Only replace if the waker is different. This will
                        // drop the other waker.
                        if !waker.will_wake(other) {
                            *other = waker.clone();
                        }
                    } else {
                        *waker_mut = Some(waker.clone());
                    }
                }
            }
            WAKING => {
                waker.wake_by_ref();
                hint::spin_loop();
            }
            state => {
                debug_assert!(state == REGISTERING || state == REGISTERING | WAKING);
            }
        }
    }

    pub fn wake_by_ref(&self) {
        if let Some(waker) = self.borrow() {
            waker.wake_by_ref();
        }
    }

    fn borrow(&self) -> Option<WakerGuard<'_>> {
        match self.state.fetch_or(WAKING, AcqRel) {
            WAITING => {
                let guard = StateGuard { state: &self.state };

                Some(WakerGuard {
                    guard,
                    waker: unsafe { (*self.waker.get()).as_ref()? },
                })
            }
            state => {
                debug_assert!(
                    state == REGISTERING || state == REGISTERING | WAKING || state == WAKING
                );
                None
            }
        }
    }
}

impl fmt::Debug for AtomicWaker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AtomicWaker")
    }
}

unsafe impl Send for AtomicWaker {}
unsafe impl Sync for AtomicWaker {}

struct StateGuard<'a> {
    state: &'a AtomicUsize,
}

impl Drop for StateGuard<'_> {
    fn drop(&mut self) {
        self.state.fetch_and(!WAKING, Release);
    }
}

struct WakerGuard<'a> {
    #[allow(unused)]
    guard: StateGuard<'a>,
    waker: &'a Waker,
}

impl Deref for WakerGuard<'_> {
    type Target = Waker;

    fn deref(&self) -> &Self::Target {
        self.waker
    }
}

struct UnlockGuard<'a> {
    state: &'a AtomicUsize,
    waker: &'a UnsafeCell<Option<Waker>>,
}

impl Drop for UnlockGuard<'_> {
    fn drop(&mut self) {
        unsafe {
            let res = self
                .state
                .compare_exchange(REGISTERING, WAITING, AcqRel, Acquire);

            if let Err(actual) = res {
                debug_assert_eq!(actual, REGISTERING | WAKING);

                if let Some(waker) = (*self.waker.get()).as_ref() {
                    waker.wake_by_ref();
                }

                self.state.swap(WAITING, AcqRel);
            }
        }
    }
}
