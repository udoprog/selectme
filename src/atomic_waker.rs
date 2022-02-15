// Copied and modified under the MIT license from the Tokio project.
//
// See: https://github.com/tokio-rs/tokio/blob/02141db/tokio/src/sync/task/atomic_waker.rs

use core::cell::UnsafeCell;
use core::hint;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::task::Waker;

pub struct AtomicWaker {
    state: AtomicUsize,
    waker: UnsafeCell<Option<Waker>>,
}

const IDLE: usize = 0;
const REGISTER: usize = 0b01;
const WAKE_BY_REF: usize = 0b10;
const LOOP: usize = 3;

impl AtomicWaker {
    pub const fn new() -> Self {
        trait AssertSync: Sync {}
        impl AssertSync for Waker {}

        Self {
            state: AtomicUsize::new(IDLE),
            waker: UnsafeCell::new(None),
        }
    }

    /// Register function that safely assumes no one else is using it, so shared
    /// access can always be completely unguarded.
    ///
    /// # Safety
    ///
    /// Only one caller may call register at a time. Otherwise a racy waker read
    /// will be performed.
    pub unsafe fn register(&self, waker: &Waker) {
        // Note that only one caller may exclusively call `register`, so
        // `self.waker` is *always* readable here since no one else tries to get
        // exclusive access and callers to `wake_by_ref` only wants to read.
        if matches!(&*self.waker.get(), Some(existing) if waker.will_wake(existing)) {
            return;
        }

        // Try to replace the waker LOOP times.
        for _ in 0..LOOP {
            let res = self
                .state
                .compare_exchange(IDLE, REGISTER, Ordering::Acquire, Ordering::Acquire)
                .unwrap_or_else(|x| x);

            if res != IDLE {
                hint::spin_loop();
                continue;
            }

            let guard = RegisterGuard { this: self };

            // Note: must not hold this reference across the unlock
            // below.
            *guard.this.waker.get() = Some(waker.clone());
            return;
        }

        // Yield and try again later!
        waker.wake_by_ref();
    }

    /// Call wake_by_ref on the currently registered waker without deregistering
    /// it.
    pub fn wake_by_ref(&self) {
        match self.state.fetch_or(WAKE_BY_REF, Ordering::AcqRel) {
            IDLE => {
                let _guard = WakeByRefGuard { state: &self.state };

                if let Some(waker) = unsafe { &*self.waker.get() } {
                    waker.wake_by_ref();
                }
            }
            state => {
                debug_assert!(
                    state == REGISTER || state == REGISTER | WAKE_BY_REF || state == WAKE_BY_REF
                );
            }
        }
    }
}

unsafe impl Send for AtomicWaker {}
unsafe impl Sync for AtomicWaker {}

struct WakeByRefGuard<'a> {
    state: &'a AtomicUsize,
}

impl Drop for WakeByRefGuard<'_> {
    fn drop(&mut self) {
        self.state.fetch_and(!WAKE_BY_REF, Ordering::Release);
    }
}

struct RegisterGuard<'a> {
    this: &'a AtomicWaker,
}

impl Drop for RegisterGuard<'_> {
    fn drop(&mut self) {
        unsafe {
            // WAKE_BY_REF interest was registered, so we have to take care of
            // it by issuing a `wake_by_ref`. We know that REGISTER is always
            // active here.
            let res = self.this.state.swap(IDLE, Ordering::AcqRel);

            if res & WAKE_BY_REF != 0 {
                debug_assert_eq!(res, REGISTER | WAKE_BY_REF);

                if let Some(waker) = &*self.this.waker.get() {
                    waker.wake_by_ref();
                }
            } else {
                debug_assert_eq!(res, REGISTER);
            }
        }
    }
}
