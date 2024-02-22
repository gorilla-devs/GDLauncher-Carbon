use std::{
    io,
    mem::{self, ManuallyDrop, MaybeUninit},
    pin::Pin,
    sync::mpsc,
    task::{Context, Poll, Waker},
};

use futures::Future;
use parking_lot::Mutex;
use tokio::io::{AsyncRead, AsyncReadExt};

/// Execute a CPU intensive task on the rayon threadpool, capturing the environment.
///
/// Warning: dropping this future will cause it to block on the dropping thread until completion.
/// SAFETY: The future must not be deleted without dropping.
pub async fn cpu_block<F: (FnOnce() -> R) + Send, R: Send>(f: F) -> R {
    enum WaitFutureWakerStage {
        NoWaker,
        Waker(Waker),
        Dropping { complete: mpsc::SyncSender<()> },
    }

    struct WaitFutureWaker {
        complete: bool,
        stage: WaitFutureWakerStage,
    }

    impl WaitFutureWaker {
        fn complete(&mut self) {
            self.complete = true;

            let mut stage = WaitFutureWakerStage::NoWaker;
            mem::swap(&mut self.stage, &mut stage);

            match stage {
                WaitFutureWakerStage::NoWaker => {}
                WaitFutureWakerStage::Waker(waker) => waker.wake(),
                WaitFutureWakerStage::Dropping { complete } => drop(complete.send(())),
            };
        }
    }

    struct WaitFuture<'a> {
        waker: &'a Mutex<WaitFutureWaker>,
    }

    impl<'a> Future for WaitFuture<'a> {
        type Output = ();

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let mut waker = self.waker.lock();
            match waker.complete {
                true => Poll::Ready(()),
                false => {
                    waker.stage = WaitFutureWakerStage::Waker(cx.waker().clone());
                    Poll::Pending
                }
            }
        }
    }

    impl<'a> Drop for WaitFuture<'a> {
        fn drop(&mut self) {
            // In the case that the future is dropped we must run it to completion or risk undefined behavior,
            // so we block until it finishes.
            let mut waker = self.waker.lock();
            if !waker.complete {
                tracing::warn!({ future_address = ?(self as *const Self) }, "CPU blocking future was dropped while running! To maintain soundness this will block an executor thread!");
                // attempt to reduce the hit on the executor
                tokio::task::block_in_place(|| {
                    let (tx, rx) = mpsc::sync_channel::<()>(1);
                    waker.stage = WaitFutureWakerStage::Dropping { complete: tx };

                    drop(waker);
                    let _ = rx.recv();
                });
                tracing::warn!({ future_address = ?(self as *const Self) }, "CPU blocking future finished running in drop. No longer blocking executor.");
            }
        }
    }

    let waker = Mutex::new(WaitFutureWaker {
        complete: false,
        stage: WaitFutureWakerStage::NoWaker,
    });

    let future = WaitFuture { waker: &waker };

    // SAFETY: static_future is never used after the wait future is woken.
    let static_waker: &'static Mutex<WaitFutureWaker> = unsafe { mem::transmute(&waker) };

    struct Sendable<T>(T);
    unsafe impl<T> Send for Sendable<T> {}

    trait Invoker: FnOnce() {
        unsafe fn call(&self);
    }

    impl<F: FnOnce()> Invoker for F {
        unsafe fn call(&self) {
            std::ptr::read(self)()
        }
    }

    let mut result = MaybeUninit::<R>::uninit();

    // ensure `f` won't be dropped after it is called
    let mut invoker = ManuallyDrop::new(|| {
        result.write(f());
    });
    let invoker_ref: &mut dyn Invoker = &mut *invoker;
    // removes all the lifetimes
    // SAFETY: invoker_ptr is never used after the wait future is woken.
    let invoker_ptr: *mut dyn Invoker = unsafe { mem::transmute(&mut *invoker_ref) };
    let invoker_sptr = Sendable(invoker_ptr);

    rayon::spawn(move || {
        let invoker_sptr = invoker_sptr;
        // SAFETY: the stack frame containing the invoker can never be deallocated before complete() is called.
        unsafe {
            (&*invoker_sptr.0).call();
        };
        static_waker.lock().complete();
    });

    // wait for completion
    future.await;

    // SAFETY: This path can only be reached after the invoker has been called.
    // On the normal path the invoker will be called by rayon before the future is woken.
    // On the drop path the future will break above here.
    unsafe { result.assume_init() }
}

pub const BUFSIZE: usize = 1024 * 8;

pub async fn buffered_digest<F: FnMut(&mut [u8]) + Send>(
    source: &mut (impl AsyncRead + Unpin),
    mut f: F,
) -> io::Result<()> {
    let mut buffer = [0u8; BUFSIZE];

    loop {
        let n = source.read(&mut buffer).await?;

        if n == 0 {
            break Ok(());
        }

        cpu_block(|| f(&mut buffer[..n])).await;
    }
}

#[macro_export]
macro_rules! blocking_cpu_join {
    { $(let $binding:ident $(: $type:ty)? = $expr:expr;)+ } => {
        let $crate::blocking_cpu_join!(!!collect $($binding,)+) = $crate::blocking_cpu_join!(!!join $($expr,)+);
        $(let $binding $(: $type)? = $binding;)+
    };

    (!!join $first:expr, $($next:expr,)+) => {
        ::rayon::join(
            || $first,
            || $crate::blocking_cpu_join!(!!join $($next,)*),
        )
    };

    (!!join $expr:expr,) => {
        $expr
    };

    (!!collect2 $($p:pat),* $(,)?) => {
        $crate::cpu_join!(!!collect $($p,)*)
    };

    (!!collect $first:pat, $($next:pat,)+) => {
        (
            $crate::blocking_cpu_join!(!!collect $first,),
            $crate::blocking_cpu_join!(!!collect $($next,)+),
        )
    };

    (!!collect $p:pat $(,)?) => {
        $p
    };
}

#[macro_export]
macro_rules! cpu_join {
    { $(let $binding:ident $(: $type:ty)? = $expr:expr;)+ } => {
        let ($($binding),+) = $crate::cpu_block(|| {
            $crate::blocking_cpu_join! {
                $(let $binding $(: $type)? = $expr;)+
            }

            ($($binding),+)
        }).await;
    };
}

#[cfg(test)]
mod test {
    use std::{
        pin::Pin,
        ptr,
        sync::atomic::{AtomicBool, Ordering},
        task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
        time::Duration,
    };

    use futures::Future;

    // taken from https://doc.rust-lang.org/stable/std/task/struct.Waker.html#method.noop
    pub fn noop_waker() -> Waker {
        const VTABLE: RawWakerVTable = RawWakerVTable::new(
            // Cloning just returns a new no-op raw waker
            |_| RAW,
            // `wake` does nothing
            |_| {},
            // `wake_by_ref` does nothing
            |_| {},
            // Dropping does nothing as we don't allocate anything
            |_| {},
        );
        const RAW: RawWaker = RawWaker::new(ptr::null(), &VTABLE);

        unsafe { Waker::from_raw(RAW) }
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn cpu_block_normal() {
        let mut value = 5;
        let future = super::cpu_block(|| {
            value += 5;
            std::thread::sleep(Duration::from_millis(100));
            value += 5;
            value + 5
        });

        // cannot access value in this scope

        let result = future.await;
        assert_eq!(value, 15); // 10 was added
        assert_eq!(result, 20); // returned value + 5
    }

    // ensure dropping the future runs it to completion.
    // not doing that is undefined behavior.
    #[test]
    #[tracing_test::traced_test]
    fn cpu_block_drop() {
        let check = AtomicBool::new(false);

        let mut future = super::cpu_block(|| {
            std::thread::sleep(Duration::from_millis(200));
            check.store(true, Ordering::SeqCst);
        });

        let waker = noop_waker();
        let mut ctx = Context::from_waker(&waker);

        let pinned_future = unsafe { Pin::new_unchecked(&mut future) };
        // manually poll the future to start it
        assert_eq!(pinned_future.poll(&mut ctx), Poll::Pending);
        // ensure the value is false at this point and was not flipped before the drop
        assert_eq!(check.load(Ordering::SeqCst), false);
        // drop the future while polling
        drop(future);
        // ensure the drop blocked and the check was flipped
        assert_eq!(check.load(Ordering::SeqCst), true);
    }
}
