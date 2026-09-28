use core::mem::MaybeUninit;
use core::pin::Pin;

use crate::move_ref::MoveRef;
use crate::new::New;
use crate::new::TryNew;
use crate::slot_storage::SlotStorageStatus;

/// Backing storage for a [`MoveRef`].
///
/// # Specification
/// - requires: created from live uninitialized storage with its matching
///   tracker.
/// - ensures: exclusive access to placement memory for the frame lifetime.
/// - provides: a backing location for one owning move-reference.
pub struct Slot<'frame, T>
{
    /// The raw underlying (possibly uninitialized) storage memory.
    memory: &'frame mut MaybeUninit<T>,
    /// Status flags for the storage which track initialization, dropping state,
    /// and reference count.
    status: SlotStorageStatus<'frame>,
}

impl<'frame, T> Slot<'frame, T>
{
    /// Construct a slot from its backing memory and tracker.
    ///
    /// # Specification
    /// - requires: `memory` is fresh storage and `status` tracks that same live
    ///   storage.
    /// - ensures: returns a unique slot view bounded by the storage lifetime.
    /// - provides: placement access to the backing memory.
    /// - panics: none.
    #[inline]
    pub(crate) fn new(
        memory: &'frame mut MaybeUninit<T>,
        status: SlotStorageStatus<'frame>,
    ) -> Self
    {
        return Self { memory, status };
    }

    /// Construct and pin `new` into the slot and return the associated owning
    /// [`MoveRef`].
    ///
    /// # Specification
    /// - requires: the slot is fresh and `new` obeys the `New` safety
    ///   requirements.
    /// - ensures: returns a pinned unique owner of the initialized referent.
    /// - provides: a reference tied to the backing storage lifetime.
    /// - panics: follows initializer and slot state checks.
    ///
    /// # Adequacy
    /// - hypothesis: premature or duplicate destruction changes the observed
    ///   count.
    /// - witness: `slot::tests::emplace_drops_the_referent_once`.
    #[inline]
    pub fn emplace<N>(
        self,
        new: N,
    ) -> Pin<MoveRef<'frame, T>>
    where
        N: New<Output = T>,
    {
        match self.try_emplace(new) {
            | Ok(pin) => return pin,
            | Err(err) => match err {},
        }
    }

    /// Try to construct and pin `new` into the slot and return the associated
    /// owning [`MoveRef`].
    ///
    /// # Specification
    /// - requires: the slot is fresh and the initializer obeys the `TryNew`
    ///   safety requirements.
    /// - ensures: success returns a pinned unique owner of initialized `T`.
    /// - provides: a pinned reference or the initializer’s exact error.
    /// - fails: returns `N::Error` on failed initialization; storage should
    ///   remain droppable.
    /// - panics: follows the initializer and storage invariant checks.
    ///
    /// # Errors
    /// - `N::Error`: the initializer could not complete.
    ///
    /// # Adequacy
    /// - hypothesis: an error without an owner should not look like a leaked
    ///   live reference.
    /// - witness: `slot::tests::try_new_error_does_not_abort_when_storage_drops` (currently ignored, fails).
    /// - witness: `slot::tests::emplace_drops_the_referent_once` (success).
    ///
    /// # Known defect
    /// On `Err`, storage drop currently detects a leaked reference counter and
    /// aborts.
    #[inline]
    pub fn try_emplace<N>(
        self,
        new: N,
    ) -> Result<Pin<MoveRef<'frame, T>>, N::Error>
    where
        N: TryNew<Output = T>,
    {
        self.status.initialize();
        // SAFETY: this slot owns fresh uninitialized storage that stays in place during
        // construction.
        let pinned = unsafe { Pin::new_unchecked(&mut *self.memory) };
        // SAFETY: TryNew receives fresh pinned storage and must initialize it on
        // success.
        unsafe {
            new.try_new(pinned)?;
        }
        // SAFETY: TryNew returned success, so the slot now contains a valid T.
        let ptr = unsafe { self.memory.assume_init_mut() };
        // SAFETY: this slot tracks the single owning MoveRef for the initialized
        // referent.
        let mov = unsafe { MoveRef::new_unchecked(ptr, self.status) };
        let pin = mov.into_pin();
        return Ok(pin);
    }

    /// Move and pin `val` into the slot and return the associated owning
    /// [`MoveRef`].
    ///
    /// # Specification
    /// - requires: the slot is fresh and uninitialized.
    /// - ensures: places `val` at a stable address and returns its only pinned
    ///   owner.
    /// - provides: an owning pinned move-reference.
    /// - panics: follows slot invariant checks.
    ///
    /// # Adequacy
    /// - hypothesis: a second destructor after storage drop changes the
    ///   observed count.
    /// - witness: `slot::tests::pin_drops_the_referent_once`.
    #[inline]
    pub fn pin(
        self,
        val: T,
    ) -> Pin<MoveRef<'frame, T>>
    {
        return self.emplace(crate::new::of(val));
    }

    /// Move `val` into the slot and return the associated owning [`MoveRef`].
    ///
    /// # Specification
    /// - requires: the slot is fresh and uninitialized.
    /// - ensures: returns the unique owning reference to `val` in this slot.
    /// - provides: an unpinned move-reference with the slot lifetime.
    /// - panics: follows slot invariant checks.
    ///
    /// # Adequacy
    /// - hypothesis: moving or duplicate destruction changes the final
    ///   destructor count.
    /// - witness: `slot::tests::put_drops_the_referent_once`.
    #[inline]
    pub fn put(
        self,
        val: T,
    ) -> MoveRef<'frame, T>
    {
        let pin = self.pin(val);
        // SAFETY: unpinning the owning reference does not move its referent in the
        // slot.
        return unsafe { Pin::into_inner_unchecked(pin) };
    }

    /// Write `val` into the slot and returns a `&mut ref` to its location and
    /// its storage status.
    ///
    /// # Specification
    /// - requires: `val` is a valid storage value with a live allocation.
    /// - ensures: stores `val` and marks the slot initialized for subsequent
    ///   ownership transfer.
    /// - provides: mutable access to the stored value and its tracker.
    /// - panics: a debug build panics if the slot was already initialized.
    #[cfg(feature = "alloc")]
    #[inline]
    pub(crate) fn write(
        self,
        val: T,
    ) -> (&'frame mut T, SlotStorageStatus<'frame>)
    {
        self.status.initialize();
        let ptr = self.memory.write(val);
        return (ptr, self.status);
    }
}

#[cfg(test)]
mod tests
{
    use core::cell::Cell;
    use core::mem::MaybeUninit;
    use core::pin::Pin;

    use crate::MoveRef;
    use crate::SlotStorage;
    use crate::SlotStorageKind;
    use crate::new;

    #[repr(transparent)]
    struct Counted<'counter>(&'counter Cell<usize>);

    impl Drop for Counted<'_>
    {
        fn drop(&mut self)
        {
            self.0.set(self.0.get().saturating_add(1));
        }
    }

    #[test]
    fn emplace_drops_the_referent_once()
    {
        let drops = Cell::new(0);
        let mut storage = SlotStorage::new(SlotStorageKind::Keep);
        let pinned = storage.slot().emplace(new::of(Counted(&drops)));
        assert_eq!(drops.get(), 0);
        drop(pinned);
        assert_eq!(drops.get(), 1);
        drop(storage);
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn pin_drops_the_referent_once()
    {
        let drops = Cell::new(0);
        let mut storage = SlotStorage::new(SlotStorageKind::Keep);
        let pinned = storage.slot().pin(Counted(&drops));
        drop(pinned);
        drop(storage);
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn put_drops_the_referent_once()
    {
        let drops = Cell::new(0);
        let mut storage = SlotStorage::new(SlotStorageKind::Keep);
        let moved = storage.slot().put(Counted(&drops));
        drop(moved);
        drop(storage);
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn into_inner_transfers_destruction_to_the_returned_value()
    {
        let drops = Cell::new(0);
        let mut storage = SlotStorage::new(SlotStorageKind::Keep);
        let moved = storage.slot().put(Counted(&drops)).into_inner();
        assert_eq!(drops.get(), 0);
        drop(storage);
        assert_eq!(drops.get(), 0);
        drop(moved);
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn release_never_implicitly_drops_the_referent()
    {
        let drops = Cell::new(0);
        let mut storage = SlotStorage::new(SlotStorageKind::Keep);
        let raw = MoveRef::release(storage.slot().pin(Counted(&drops)));
        assert_eq!(drops.get(), 0);
        // SAFETY: raw points into live storage, and release transferred destruction to
        // us.
        unsafe {
            core::ptr::drop_in_place(raw);
        }
        assert_eq!(drops.get(), 1);
        drop(storage);
        assert_eq!(drops.get(), 1);
    }
    #[cfg(miri)]
    #[test]
    #[ignore = "Drop storage currently destroys a directly owned referent twice"]
    fn dropping_direct_value_with_drop_kind_must_not_destroy_twice()
    {
        let drops = Cell::new(0);
        let mut storage = SlotStorage::new(SlotStorageKind::Drop);
        let moved = storage.slot().put(Counted(&drops));
        drop(moved);
        assert_eq!(drops.get(), 1);
        drop(storage);
        assert_eq!(drops.get(), 1);
    }

    #[derive(Debug, PartialEq)]
    struct Rejected;

    struct RejectAfterInit;

    impl new::TryNew for RejectAfterInit
    {
        type Output = u8;
        type Error = Rejected;

        /// Return an error even though the placement was initialized.
        ///
        /// # Specification
        /// - ensures: writes a valid value to the supplied pinned storage.
        /// - fails: always returns `Rejected`.
        /// - panics: none.
        /// - unsafe invariants: initializes fresh storage without moving it.
        ///
        /// # Safety
        /// The caller supplies fresh pinned storage.
        unsafe fn try_new(
            self,
            mut this: Pin<&mut MaybeUninit<Self::Output>>,
        ) -> Result<(), Self::Error>
        {
            this.set(MaybeUninit::new(7_u8));
            Err(Rejected)
        }
    }

    #[test]
    #[ignore = "fallible initialization currently leaves the slot marked as leaking"]
    fn try_new_error_does_not_abort_when_storage_drops()
    {
        let mut storage = SlotStorage::new(SlotStorageKind::Keep);
        let result = storage.slot().try_emplace(RejectAfterInit);
        assert!(matches!(result, Err(Rejected)));
        drop(result);
        drop(storage);
    }
}
