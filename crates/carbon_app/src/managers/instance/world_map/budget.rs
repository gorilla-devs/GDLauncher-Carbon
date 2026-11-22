//! What the map's paintings running at once may hold: a few at a time, and no more bytes of
//! region files together than a budget, so a world of huge files paints them one by one.

use std::sync::Arc;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// How many regions are painted at once across the launcher.
pub(crate) const PAINTING_AT_ONCE: usize = 3;

/// The bytes of region files the paintings running at once hold together. A region of a few
/// MiB, as the game writes them, leaves room for the others.
pub(crate) const PAINTING_BYTES: u64 = 256 * 1024 * 1024;

/// The budget is counted in pieces of this many bytes.
const UNIT: u64 = 1024;

pub(crate) struct PaintingBudget {
    count: Arc<Semaphore>,
    bytes: Arc<Semaphore>,
    at_once: usize,
    units: u32,
}

impl std::fmt::Debug for PaintingBudget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PaintingBudget")
    }
}

/// A painting let in; it holds its share of the budget until dropped.
pub(super) struct Admitted {
    _count: OwnedSemaphorePermit,
    _bytes: OwnedSemaphorePermit,
}

impl PaintingBudget {
    pub(crate) fn new(at_once: usize, bytes: u64) -> PaintingBudget {
        let units = u32::try_from(bytes.div_ceil(UNIT))
            .unwrap_or(u32::MAX)
            .max(1);
        PaintingBudget {
            count: Arc::new(Semaphore::new(at_once)),
            bytes: Arc::new(Semaphore::new(units as usize)),
            at_once,
            units,
        }
    }

    /// Waits until a painting that reads `bytes` of files may start. One that needs more than
    /// the whole budget waits for every other to end and then paints alone. The waits are
    /// first come, first served, so none is passed over for good; one dropped leaves its place.
    pub(super) async fn admit(&self, bytes: u64) -> Admitted {
        // Always the count first: a painting holding a share of bytes never waits for a count.
        let count = self
            .count
            .clone()
            .acquire_owned()
            .await
            .expect("the map painting count is never closed");
        let units = u32::try_from(bytes.div_ceil(UNIT))
            .unwrap_or(u32::MAX)
            .clamp(1, self.units);
        let bytes = self
            .bytes
            .clone()
            .acquire_many_owned(units)
            .await
            .expect("the map painting bytes are never closed");
        Admitted {
            _count: count,
            _bytes: bytes,
        }
    }

    /// Whether no painting holds any of it.
    #[cfg(test)]
    pub(crate) fn idle(&self) -> bool {
        self.count.available_permits() == self.at_once
            && self.bytes.available_permits() == self.units as usize
    }
}

#[cfg(test)]
mod tests {
    use std::task::Poll;

    use futures::poll;

    use super::*;

    const KIB: u64 = 1024;

    fn budget() -> PaintingBudget {
        PaintingBudget::new(3, 100 * KIB)
    }

    #[tokio::test]
    async fn three_small_paintings_go_at_once_and_a_fourth_waits() {
        let budget = budget();
        let mut held = Vec::new();
        for _ in 0..3 {
            let mut admit = Box::pin(budget.admit(10 * KIB));
            match poll!(&mut admit) {
                Poll::Ready(admitted) => held.push(admitted),
                Poll::Pending => panic!("a small painting waited"),
            }
        }
        let mut fourth = Box::pin(budget.admit(KIB));
        assert!(poll!(&mut fourth).is_pending());
        held.pop();
        assert!(poll!(&mut fourth).is_ready());
    }

    #[tokio::test]
    async fn a_large_painting_waits_for_room_and_holds_the_others_back() {
        let budget = budget();
        let first = budget.admit(30 * KIB).await;
        let second = budget.admit(30 * KIB).await;
        let mut large = Box::pin(budget.admit(60 * KIB));
        assert!(poll!(&mut large).is_pending());
        // Room for a small one, but it comes after the large one.
        let mut small = Box::pin(budget.admit(KIB));
        assert!(poll!(&mut small).is_pending());
        drop(first);
        let large = match poll!(&mut large) {
            Poll::Ready(admitted) => admitted,
            Poll::Pending => panic!("the large painting still waits"),
        };
        assert!(poll!(&mut small).is_ready());
        drop((second, large, small));
        assert!(budget.idle());
    }

    #[tokio::test]
    async fn a_painting_larger_than_the_budget_paints_alone() {
        let budget = budget();
        let small = budget.admit(KIB).await;
        let mut huge = Box::pin(budget.admit(10 * 100 * KIB));
        assert!(poll!(&mut huge).is_pending());
        drop(small);
        let huge = match poll!(&mut huge) {
            Poll::Ready(admitted) => admitted,
            Poll::Pending => panic!("a painting larger than the budget never starts"),
        };
        let mut other = Box::pin(budget.admit(KIB));
        assert!(poll!(&mut other).is_pending());
        drop(huge);
        assert!(poll!(&mut other).is_ready());
    }

    #[tokio::test]
    async fn a_dropped_wait_gives_its_place_up() {
        let budget = budget();
        let held = budget.admit(50 * KIB).await;
        let mut large = Box::pin(budget.admit(80 * KIB));
        assert!(poll!(&mut large).is_pending());
        drop(large);
        // Nothing waits before it any more.
        let mut small = Box::pin(budget.admit(10 * KIB));
        assert!(poll!(&mut small).is_ready());
        drop((held, small));
        assert!(budget.idle());
    }
}
