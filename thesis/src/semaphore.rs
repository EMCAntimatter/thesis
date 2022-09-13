use std::ops::SubAssign;

use parking_lot::Mutex;

pub struct SpinSemaphore {
    max: u32,
    var: Mutex<u32>,
}

impl SpinSemaphore {
    pub const fn new(amount: u32) -> SpinSemaphore {
        SpinSemaphore {
            max: amount,
            var: Mutex::new(amount),
        }
    }

    pub fn take_n(&self, n: u32) -> SpinSemaphoreGuard {
        loop {
            let mut val = self.var.lock();
            if let Some(_) = val.checked_sub(n) {
                val.sub_assign(n);
                return SpinSemaphoreGuard {
                    count: n,
                    semaphore: &self,
                };
            }
            drop(val);
        }
    }

    pub fn take_1(&self) -> SpinSemaphoreGuard {
        self.take_n(1)
    }

    pub fn take_max(&self) -> SpinSemaphoreGuard {
        self.take_n(self.max)
    }

    pub fn with_guard_n(&self, n: u32, f: impl FnOnce()) {
        let guard = self.take_n(n);
        f();
        drop(guard);
    }

    pub fn with_guard_1(&self, f: impl FnOnce()) {
        self.with_guard_n(1, f)
    }
}

unsafe impl Sync for SpinSemaphore {}

pub struct SpinSemaphoreGuard<'semaphore> {
    count: u32,
    semaphore: &'semaphore SpinSemaphore,
}

impl<'semaphore> Drop for SpinSemaphoreGuard<'semaphore> {
    fn drop(&mut self) {
        let mut val = self.semaphore.var.lock();
        *val += self.count;
    }
}
