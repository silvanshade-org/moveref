#[cfg(feature = "alloc")]
use core::mem::MaybeUninit;
use core::ops::Deref;
use core::pin::Pin;

use crate::deref_move::DerefMove;
use crate::move_ref::MoveRef;
use crate::slot::Slot;

/// A trait for transforming a [`Deref`] type into a pinned [`MoveRef`] with
/// respect to a specified backing storage type [`IntoMove::Storage`].
///
/// # Specification
/// - requires: implementations transfer unique ownership into the backing
///   storage.
/// - ensures: the referent remains at its original pinned address.
/// - provides: a pinned owning move-reference.
pub trait IntoMove: Deref + Sized
{
    /// Storage needed to retain ownership of the referent or its allocation.
    type Storage: Sized;

    /// Consume `self` and create a pinned [`MoveRef`] with `self`'s contents
    /// placed into `storage`.
    ///
    /// # Specification
    /// - requires: `self` owns the referent at a valid address.
    /// - ensures: consumes `self` while retaining the referent address.
    /// - provides: a pinned `MoveRef` tied to `storage`.
    /// - panics: follows the implementation’s storage operations.
    fn into_move<'frame>(
        self,
        storage: Slot<'frame, Self::Storage>,
    ) -> Pin<MoveRef<'frame, Self::Target>>
    where
        Self: 'frame;
}

#[cfg(feature = "alloc")]
impl<T> IntoMove for crate::Box<T>
{
    type Storage = crate::Box<MaybeUninit<T>>;

    #[inline]
    /// Transfer a box into pinned owning storage without moving its referent.
    ///
    /// # Specification
    /// - requires: the box uniquely owns its initialized referent.
    /// - ensures: the returned pinned reference keeps the referent at its
    ///   allocation address.
    /// - provides: pinned ownership backed by the box allocation.
    /// - panics: follows storage state checks.
    fn into_move<'frame>(
        self,
        storage: Slot<'frame, Self::Storage>,
    ) -> Pin<MoveRef<'frame, Self::Target>>
    where
        Self: 'frame,
    {
        return MoveRef::into_pin(self.deref_move(storage));
    }
}

impl<T: ?Sized> IntoMove for MoveRef<'_, T>
{
    type Storage = ();

    #[inline]
    /// Pin an existing move-reference after transferring its ownership.
    ///
    /// # Specification
    /// - requires: `self` owns its live referent.
    /// - ensures: preserves the referent address and unique ownership.
    /// - provides: a pinned move-reference.
    /// - panics: none.
    fn into_move<'frame>(
        self,
        storage: Slot<'frame, Self::Storage>,
    ) -> Pin<MoveRef<'frame, Self::Target>>
    where
        Self: 'frame,
    {
        return MoveRef::into_pin(self.deref_move(storage));
    }
}

impl<P: DerefMove> IntoMove for Pin<P>
{
    type Storage = P::Storage;

    #[inline]
    /// Convert a pinned owning pointer without relocating its referent.
    ///
    /// # Specification
    /// - requires: `P` is a uniquely owning movable pointer implementation.
    /// - ensures: returns a pinned move-reference at the referent’s current
    ///   address.
    /// - provides: a unique owner whose storage is governed by `P::Storage`.
    /// - panics: follows `P::deref_move`.
    fn into_move<'frame>(
        self,
        storage: Slot<'frame, Self::Storage>,
    ) -> Pin<MoveRef<'frame, Self::Target>>
    where
        Self: 'frame,
    {
        // SAFETY: the owning P is immediately consumed by deref_move, which transfers
        // its referent.
        let inner = unsafe { Self::into_inner_unchecked(self) };
        let this = P::deref_move(inner, storage);
        return MoveRef::into_pin(this);
    }
}

#[cfg(test)]
mod tests
{
    use crate::*;

    mod coverage
    {
        use super::*;

        mod into_move
        {
            use super::*;

            const VAL: &str = "value";

            #[cfg(feature = "alloc")]
            #[test]
            fn r#box()
            {
                let kind = SlotStorageKind::Drop;
                let mut storage = SlotStorage::new(kind);
                let slot = storage.slot();
                let mref = IntoMove::into_move(Box::new(VAL), slot);
                assert_eq!(VAL, *mref);
            }

            #[test]
            fn move_ref()
            {
                let kind = SlotStorageKind::Drop;
                let mut storage = SlotStorage::new(kind);
                let slot = storage.slot();
                bind!(val: MoveRef<'_, &str> = &move VAL);
                let mref = IntoMove::into_move(val, slot);
                assert_eq!(VAL, *mref);
            }

            #[test]
            fn pin()
            {
                let kind = SlotStorageKind::Drop;
                let mut storage = SlotStorage::new(kind);
                let slot = storage.slot();
                let mref = IntoMove::into_move(Box::pin(VAL), slot);
                assert_eq!(VAL, *mref);
            }
        }
    }
}
