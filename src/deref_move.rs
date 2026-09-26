#[cfg(feature = "alloc")]
use core::mem::MaybeUninit;
use core::ops::DerefMut;

use crate::into_move::IntoMove;
use crate::move_ref::MoveRef;
use crate::slot::Slot;

/// Dereferencing move operations for [`MoveRef`].
///
/// This trait serves a similar purpose for [`MoveRef`] as
/// [`Deref`](core::ops::Deref) does for normal references.
///
/// In order to implement this trait, the `Self` pointer type must be the
/// *unique owner* of its referent, such that dropping `Self` would cause its
/// referent's destructor to run.
///
/// This is a subtle condition that depends upon the semantics of the `Self`
/// pointer type and must be verified by the implementer, hence the unsafety.
///
/// Examples:
///
/// - [`MoveRef<T>`] implements [`DerefMove`] by definition.
/// - [`Box<T>`](crate::Box<T>) implements [`DerefMove`] because when it drops
///   it destructs `T`.
/// - `&mut T` does *not* implement [`DerefMove`] because it is non-owning.
/// - [`Arc<T>`](crate::Arc<T>) does *not* implement [`DerefMove`] because it is
///   not *uniquely* owning.
/// - [`Rc<T>`](crate::Rc<T>) does *not* implement [`DerefMove`] because it is
///   not *uniquely* owning.
/// - [`Pin<P>`](core::pin::Pin<T>) given `P: DerefMove`, implements
///   [`DerefMove`] only when `P::Target: Unpin`, because `DerefMove: DerefMut`
///   and `Pin<P>: DerefMut` requires `P::Target: Unpin`.
///
/// # Specification
/// - requires: implementations uniquely own a live referent until transfer or
///   destruction.
/// - ensures: transfer preserves the referent address and single-destruction
///   ownership.
/// - provides: an owning move reference rather than a borrowed mutable
///   reference.
/// - unsafe invariants: no other owner may destroy the referent after transfer.
///
/// # Adequacy
/// - hypothesis: duplicate ownership would change the destructor count when
///   both values leave scope.
/// - witness: `deref_move::tests::boxed_referent_moves_without_changing_address_or_duplicate_drop`.
///
/// # Safety
///
/// Correctness for [`DerefMove`] impls require that the unique ownership
/// invariant of [`MoveRef`] is upheld. In particular, the following function
/// must preserve that invariant:
/// ```
/// # use moveref::{DerefMove, MoveRef, bind};
/// fn move_out_of<P>(ptr: P) -> P::Target
/// where
///     P: DerefMove,
///     P::Target: Sized,
/// {
///     bind!(mvp = &move *ptr);
///     MoveRef::into_inner(mvp)
/// }
/// ```
pub unsafe trait DerefMove: DerefMut + IntoMove
{
    /// Construct a [`MoveRef`] by dereferencing `self` and moving its contents
    /// into `storage`.
    ///
    /// # Specification
    /// - requires: `self` is the unique owning pointer to its referent.
    /// - ensures: the referent keeps its address and its destructor runs
    ///   exactly once.
    /// - provides: a `MoveRef` whose lifetime is tied to `storage`.
    /// - panics: follows the storage and owning-pointer operations.
    ///
    /// # Adequacy
    /// - hypothesis: retaining a second owner causes early or duplicate
    ///   destruction.
    /// - witness: `deref_move::tests::boxed_referent_moves_without_changing_address_or_duplicate_drop`.
    fn deref_move<'frame>(
        self,
        storage: Slot<'frame, Self::Storage>,
    ) -> MoveRef<'frame, Self::Target>
    where
        Self: 'frame;
}

#[cfg(feature = "alloc")]
// SAFETY: a Box uniquely owns its allocation; moving it into the slot transfers
// that ownership.
unsafe impl<T> DerefMove for crate::Box<T>
{
    /// Transfer a boxed referent while keeping its allocation alive through the
    /// slot.
    ///
    /// # Specification
    /// - requires: the boxed referent is initialized and uniquely owned.
    /// - ensures: retains the referent address and drops it once through the
    ///   owning reference.
    /// - provides: a `MoveRef` backed by storage that owns the box allocation.
    /// - panics: follows allocation and slot state checks.
    ///
    /// # Adequacy
    /// - hypothesis: relocating the referent or dropping twice would alter
    ///   address or drop count.
    /// - witness: `deref_move::tests::boxed_referent_moves_without_changing_address_or_duplicate_drop`.
    #[inline]
    fn deref_move<'frame>(
        self,
        storage: Slot<'frame, Self::Storage>,
    ) -> MoveRef<'frame, Self::Target>
    where
        Self: 'frame,
    {
        let cast = Self::into_raw(self).cast::<MaybeUninit<T>>();
        // SAFETY: the pointer came from this Box and has the same allocation and
        // layout.
        let cast = unsafe { crate::Box::from_raw(cast) };
        let (ptr, status) = storage.write(cast);
        // SAFETY: the original Box held an initialized T and no write changed its
        // bytes.
        let ptr = unsafe { ptr.assume_init_mut() };
        // SAFETY: the Box's referent is now uniquely owned by this MoveRef and the slot
        // tracks it.
        return unsafe { MoveRef::new_unchecked(ptr, status) };
    }
}

// SAFETY: a MoveRef uniquely owns its referent by construction; transferring it
// preserves ownership.
unsafe impl<T: ?Sized> DerefMove for MoveRef<'_, T>
{
    /// Transfer an existing owner into the requested lifetime without moving
    /// its referent.
    ///
    /// # Specification
    /// - requires: `self` uniquely owns the live referent.
    /// - ensures: the returned reference owns the same referent at the same
    ///   address.
    /// - provides: the original owner with a lifetime bounded by `storage`.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: a second owner or early destructor would change the
    ///   observed drop count.
    /// - witness: `deref_move::tests::moving_a_move_ref_transfers_its_existing_owner`.
    #[inline]
    fn deref_move<'frame>(
        self,
        _storage: Slot<'frame, Self::Storage>,
    ) -> MoveRef<'frame, Self::Target>
    where
        Self: 'frame,
    {
        return self;
    }
}

#[cfg(test)]
mod tests
{
    use core::cell::Cell;

    use crate::DerefMove;
    use crate::SlotStorage;
    use crate::SlotStorageKind;

    #[repr(transparent)]
    struct Counted<'counter>(&'counter Cell<usize>);

    impl Drop for Counted<'_>
    {
        fn drop(&mut self)
        {
            self.0.set(self.0.get().saturating_add(1));
        }
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn boxed_referent_moves_without_changing_address_or_duplicate_drop()
    {
        let drops = Cell::new(0);
        let owned = crate::Box::new(Counted(&drops));
        let address = core::ptr::from_ref(&*owned);
        let mut storage = SlotStorage::new(SlotStorageKind::Drop);
        let moved = DerefMove::deref_move(owned, storage.slot());
        assert_eq!(moved.as_ptr(), address);
        drop(moved);
        assert_eq!(drops.get(), 1);
        drop(storage);
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn moving_a_move_ref_transfers_its_existing_owner()
    {
        let drops = Cell::new(0);
        let mut storage = SlotStorage::new(SlotStorageKind::Keep);
        let first = storage.slot().put(Counted(&drops));
        let address = first.as_ptr();
        let mut unused = SlotStorage::new(SlotStorageKind::Keep);
        let moved = DerefMove::deref_move(first, unused.slot());
        assert_eq!(moved.as_ptr(), address);
        drop(moved);
        drop(unused);
        drop(storage);
        assert_eq!(drops.get(), 1);
    }
}
