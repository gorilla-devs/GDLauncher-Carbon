use std::{
    mem::ManuallyDrop,
    sync::atomic::{self, AtomicBool},
};

/// Primitive for sending data by value that may be retrieved one time from an immutable reference.
pub struct OnceSend<T> {
    data: ManuallyDrop<T>,
    taken: AtomicBool,
}

impl<T> OnceSend<T> {
    pub fn new(data: T) -> Self {
        Self {
            data: ManuallyDrop::new(data),
            taken: AtomicBool::new(false),
        }
    }

    /// Returns the contained value if it has not been taken already.
    pub fn take(&self) -> Option<T> {
        match self.taken.swap(true, atomic::Ordering::Relaxed) {
            // SAFETY: `self.taken` is atomic and ensures only one thread may take this OnceSend
            // at a time. The value of `data` is read only, and only read once, as taken has already been atomically set to true.
            // We leave the data in `self.data` and pretend that it does not exist, including not calling drop.
            false => Some(unsafe { self.get_unchecked() }),
            true => None,
        }
    }

    /// Unsafely returns the contained data by value.
    ///
    /// # Safety
    /// This method can only be called once per instance.
    unsafe fn get_unchecked(&self) -> T {
        // SAFETY: `T` and `ManuallyDrop<T>` share a repr.
        unsafe { (&self.data as *const _ as *const T).read() }
    }
}

impl<T> Drop for OnceSend<T> {
    fn drop(&mut self) {
        // Only drop the contained data if it has not already been taken.
        if !self.taken.load(atomic::Ordering::Relaxed) {
            // SAFETY: `drop()` takes &mut, so no other thread can concurrently call `take()` or `drop()`.
            drop(unsafe { self.get_unchecked() });
        }
    }
}

#[cfg(test)]
mod test {
    use std::cell::Cell;

    use crate::once_send::OnceSend;

    struct TestType<'a> {
        dropped: &'a Cell<bool>,
    }

    impl Drop for TestType<'_> {
        fn drop(&mut self) {
            self.dropped.set(true);
        }
    }

    #[test]
    fn take_once_only() {
        let once = OnceSend::new(());
        assert_eq!(once.take(), Some(()));
        assert_eq!(once.take(), None);
    }

    #[test]
    fn drop_if_not_taken() {
        let dropped = Cell::new(false);
        drop(OnceSend::new(TestType { dropped: &dropped }));
        assert_eq!(dropped.get(), true);
    }

    #[test]
    fn not_drop_if_taken() {
        let dropped = Cell::new(false);
        let once = OnceSend::new(TestType { dropped: &dropped });
        let _ = once.take();
        assert_eq!(dropped.get(), true);
        dropped.set(false);
        drop(once);
        assert_eq!(dropped.get(), false);
    }
}
