use core::ops::Deref;
use core::ops::DerefMut;
use core::pin::Pin;

use crate::slot_storage::SlotStorageStatus;

/// A "reference" type which *uniquely* owns its referent type `T` with respect
/// to external storage with lifetime `'frame`.
///
/// Conceptually, it has these characteristics:
///
/// - similar to `&'frame mut` because it *uniquely* references other data with
///   lifetime `'frame`
/// - similar to `Box` because it is *owning*
///
/// Unlike `&mut` or [`Box`](crate::Box), it uses a backing
/// [`Slot`](crate::Slot) to track exclusive ownership and the referent’s
/// destructor. The storage may also own a separate allocation that must be
/// freed after the referent is dropped.
///
/// A motivating example for [`MoveRef`] is the concept of
/// placement-initialization in C++:
///
/// Imagine we define FFI bindings for a C++ class we intend to use in Rust.
///
/// Creating instances for this class on the heap is straightforward and well
/// understood: we can use raw pointers and eventually either convert to a
/// reference or [`Box`](crate::Box).
///
/// Creating instances for this class on the stack is more difficult. We can use
/// [`MaybeUninit`](core::mem::MaybeUninit) to create a chunk of data and
/// initialize into that.
///
/// But we have to be particularly careful when using the result because in
/// Rust, data moves by default, rather than copies by default as in C++. So any
/// access of the data in Rust could potentially move the data out from under
/// some expected location in C++ and cause a crash when execution proceeds
/// again in C++.
///
/// So we need a type which acts like a (mutable) reference but does not let us
/// move simply by accessing it. This would be similar to a [`Pin<&mut T>`],
/// where the [`Pin`] prevents movement, but the inner `&mut` still allows
/// mutation.
///
/// But we also want the possibility to *actually* move the data in some cases,
/// like we would explicitly do in C++ with a move constructor or move
/// assignment operation.
///
/// This interface is exactly what [`MoveRef`] provides, along with
/// [`DerefMove`](crate::DerefMove).
///
/// # Specification
/// - requires: the referent lives in a slot for the entire frame.
/// - ensures: exactly one owner is responsible for the referent’s destructor
///   until release.
/// - provides: exclusive access without implicitly moving the referent.
pub struct MoveRef<'frame, T: ?Sized>
{
    /// The underlying mutable reference with referent stored in some external
    /// [`Slot`](crate::Slot).
    ptr: &'frame mut T,
    /// Status flags for the storage which track initialization, dropping state,
    /// and reference count.
    status: SlotStorageStatus<'frame>,
}

impl<T: ?Sized + core::fmt::Debug> core::fmt::Debug for MoveRef<'_, T>
{
    /// Format the referent with its `Debug` implementation.
    ///
    /// # Specification
    /// - ensures: delegates formatting to the referent.
    /// - provides: the formatter result.
    /// - panics: follows the referent formatter.
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        return core::fmt::Debug::fmt(self.ptr, f);
    }
}

impl<T: ?Sized> Deref for MoveRef<'_, T>
{
    type Target = T;

    /// Borrow the live referent without transferring its ownership.
    ///
    /// # Specification
    /// - ensures: returns shared access to the referent.
    /// - provides: a borrow limited by `self`.
    /// - panics: none.
    #[inline]
    fn deref(&self) -> &Self::Target
    {
        return self.ptr;
    }
}

impl<T: ?Sized> DerefMut for MoveRef<'_, T>
{
    /// Mutably borrow the live referent without transferring its ownership.
    ///
    /// # Specification
    /// - ensures: returns exclusive mutable access to the referent.
    /// - provides: a mutable borrow limited by `self`.
    /// - panics: none.
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        return self.ptr;
    }
}

impl<T: ?Sized> Drop for MoveRef<'_, T>
{
    /// Destroy an unreleased referent exactly once.
    ///
    /// # Specification
    /// - ensures: an unreleased owner terminates its tracker and destructs the
    ///   referent.
    /// - provides: no destructor action after explicit release.
    /// - panics: follows the referent’s destructor or a failed storage
    ///   invariant.
    ///
    /// # Adequacy
    /// - hypothesis: missing or duplicate destruction changes the observed
    ///   counter.
    /// - witness: `slot::tests::put_drops_the_referent_once`.
    #[inline]
    fn drop(&mut self)
    {
        if self.status.is_released() {
            return;
        }
        self.status.terminate();
        // SAFETY: this MoveRef uniquely owns the initialized referent until it is
        // dropped.
        unsafe { core::ptr::drop_in_place(self.ptr) }
    }
}

impl<'frame, T: ?Sized> MoveRef<'frame, T>
{
    /// Create a new unchecked [`MoveRef`] from a mutable ref and
    /// [`SlotStorageStatus`].
    ///
    /// # Specification
    /// - requires: `ptr` points to initialized unique ownership tracked by
    ///   `status`.
    /// - ensures: the returned reference owns that referent until drop or
    ///   release.
    /// - provides: an owner tied to the slot’s lifetime.
    /// - panics: none.
    /// - unsafe invariants: no other owner may access or destroy the referent
    ///   after transfer.
    ///
    /// # Safety
    ///
    /// `ptr` must be an initialized, uniquely owned referent in the live slot
    /// tracked by `status`. No other owner may drop or move it while this
    /// reference exists.
    #[inline]
    pub(crate) unsafe fn new_unchecked(
        ptr: &'frame mut T,
        status: SlotStorageStatus<'frame>,
    ) -> Self
    {
        return Self { ptr, status };
    }

    /// Transform a [`MoveRef<T>`] into a [`Pin<MoveRef<T>>`]. Its referent
    /// stays at its pinned address in the backing [`Slot`](crate::Slot)
    /// until the reference and slot are dropped.
    ///
    /// # Specification
    /// - requires: the owning reference’s slot remains alive.
    /// - ensures: pins the owning reference without relocating its referent.
    /// - provides: a pinned reference with the same destructor obligation.
    /// - panics: none.
    #[must_use]
    #[inline]
    pub fn into_pin(self) -> Pin<Self>
    {
        // SAFETY: the referent stays in its backing slot.
        return unsafe { Pin::new_unchecked(self) };
    }

    /// Consume a [`Pin<Self>`] and return a raw `*mut T`. This operation
    /// inhibits destruction of `T` by implicit [`Drop`] and the caller
    /// becomes responsible for eventual explicit destruction and cleanup,
    /// otherwise the memory will leak.
    ///
    /// # Specification
    /// - requires: the pinned reference uniquely owns a live initialized
    ///   referent.
    /// - ensures: no implicit destructor runs through the reference or its
    ///   backing slot.
    /// - provides: the referent pointer; the caller takes over destruction and
    ///   cleanup.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: release followed by explicit destruction would reveal any
    ///   implicit drop.
    /// - witness: `slot::tests::release_never_implicitly_drops_the_referent`.
    #[inline]
    #[must_use]
    pub fn release(pin: Pin<Self>) -> *mut T
    {
        // SAFETY: release only removes the pin after consuming the owning reference.
        let mov = unsafe { Pin::into_inner_unchecked(pin) };
        // SAFETY: the caller takes over destruction after release.
        unsafe {
            mov.status.release();
        }
        return mov.ptr;
    }
}

impl<T> MoveRef<'_, T>
{
    /// Move the referent out and transfer its destructor obligation to the
    /// returned value.
    ///
    /// # Specification
    /// - requires: the reference uniquely owns an initialized sized referent.
    /// - ensures: neither reference nor backing slot drops the moved-out
    ///   referent.
    /// - provides: ownership of `T` to the caller.
    /// - panics: follows storage state checks.
    ///
    /// # Adequacy
    /// - hypothesis: an early or duplicate drop would change the counter
    ///   before/after return.
    /// - witness: `slot::tests::into_inner_transfers_destruction_to_the_returned_value`.
    #[must_use]
    #[inline]
    pub fn into_inner(self) -> T
    {
        // SAFETY: pinning the MoveRef is sound because its referent stays in the slot.
        let pin = unsafe { Pin::new_unchecked(self) };
        let ptr = MoveRef::release(pin);
        // SAFETY: release transfers ownership out of the pinned MoveRef.
        return unsafe { core::ptr::read(ptr) };
    }

    /// Return a raw pointer to the initialized referent.
    ///
    /// # Specification
    /// - ensures: the pointer identifies the live referent without transferring
    ///   ownership.
    /// - provides: read-only raw access while this owner remains valid.
    /// - panics: none.
    #[must_use]
    #[inline]
    pub fn as_ptr(&self) -> *const T
    {
        return self.ptr;
    }

    /// Return a mutable raw pointer to the initialized referent.
    ///
    /// # Specification
    /// - requires: callers of the returned pointer preserve exclusive access.
    /// - ensures: the pointer identifies the live referent without transferring
    ///   ownership.
    /// - provides: mutable raw access while this owner remains valid.
    /// - panics: none.
    #[must_use]
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T
    {
        return self.ptr;
    }
}

impl<S, T: ?Sized + PartialEq<S>> PartialEq<MoveRef<'_, S>> for MoveRef<'_, T>
{
    /// Compare the owned referents rather than their addresses.
    ///
    /// # Specification
    /// - ensures: returns equality of the referents.
    /// - provides: the same equality relation as `T` and `S`.
    /// - panics: follows referent comparison.
    ///
    /// # Adequacy
    /// - hypothesis: pointer identity differs from equal values in distinct
    ///   slots.
    /// - witness: `move_ref::test::coverage::move_ref::partial_eq`.
    #[inline]
    fn eq(
        &self,
        other: &MoveRef<'_, S>,
    ) -> bool
    {
        return self.ptr == other.ptr;
    }
}

/// Preserve the referent’s equality relation.
///
/// # Specification
/// trivial.
impl<T: Eq> Eq for MoveRef<'_, T>
{
}

impl<S, T: ?Sized + PartialOrd<S>> PartialOrd<MoveRef<'_, S>> for MoveRef<'_, T>
{
    /// Compare the owned referents with partial ordering.
    ///
    /// # Specification
    /// - ensures: returns the partial ordering of the referents.
    /// - provides: the same comparison result as `T` and `S`.
    /// - panics: follows referent comparison.
    #[inline]
    fn partial_cmp(
        &self,
        other: &MoveRef<'_, S>,
    ) -> Option<core::cmp::Ordering>
    {
        return self.ptr.partial_cmp(&other.ptr);
    }

    /// Compare whether this referent is less than another.
    ///
    /// # Specification
    /// trivial.
    #[inline]
    fn lt(
        &self,
        other: &MoveRef<'_, S>,
    ) -> bool
    {
        return self.ptr.lt(&other.ptr);
    }

    /// Compare whether this referent is less than or equal to another.
    ///
    /// # Specification
    /// trivial.
    #[inline]
    fn le(
        &self,
        other: &MoveRef<'_, S>,
    ) -> bool
    {
        return self.ptr.le(&other.ptr);
    }

    /// Compare whether this referent is greater than another.
    ///
    /// # Specification
    /// trivial.
    #[inline]
    fn gt(
        &self,
        other: &MoveRef<'_, S>,
    ) -> bool
    {
        return self.ptr.gt(&other.ptr);
    }

    /// Compare whether this referent is greater than or equal to another.
    ///
    /// # Specification
    /// trivial.
    #[inline]
    fn ge(
        &self,
        other: &MoveRef<'_, S>,
    ) -> bool
    {
        return self.ptr.ge(&other.ptr);
    }
}

impl<T: Ord> Ord for MoveRef<'_, T>
{
    /// Compare the owned referents with total ordering.
    ///
    /// # Specification
    /// trivial.
    #[inline]
    fn cmp(
        &self,
        other: &Self,
    ) -> core::cmp::Ordering
    {
        return self.ptr.cmp(&other.ptr);
    }
}

impl<T: ?Sized + core::hash::Hash> core::hash::Hash for MoveRef<'_, T>
{
    /// Hash the referent, not the move-reference address.
    ///
    /// # Specification
    /// - ensures: writes the referent’s hash into `state`.
    /// - provides: the same hash relation as `T`.
    /// - panics: follows the referent hasher.
    #[inline]
    fn hash<H>(
        &self,
        state: &mut H,
    ) where
        H: core::hash::Hasher,
    {
        self.ptr.hash(state);
    }
}

#[cfg(test)]
mod test
{
    use super::*;
    use crate::*;

    #[cfg(feature = "alloc")]
    #[test]
    fn deref_move_of_move_ref()
    {
        bind!(x: MoveRef<'_, crate::Box<i32>> = &move crate::Box::new(5_i32));
        bind!(y: MoveRef<'_, crate::Box<i32>> = &move *x);
        let z = y;
        assert_eq!(**z, 5_i32);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn deref_move_of_box()
    {
        let x = crate::Box::new(5_i32);
        bind!(y: MoveRef<'_, i32> = &move *x);
        let z = y;
        assert_eq!(*z, 5_i32);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn move_ref_into_inner()
    {
        bind!(x: MoveRef<'_, crate::Box<i32>> = &move crate::Box::new(5_i32));
        let y = x.into_inner();
        assert_eq!(*y, 5_i32);
    }

    #[test]
    #[expect(
        clippy::mem_forget,
        reason = "this test checks leaked MoveRef detection"
    )]
    #[should_panic(expected = "a critical reference counter at")]
    fn forget_move_ref()
    {
        bind!(x: MoveRef<'_, i32> = &move 42_i32);
        core::mem::forget(x);
    }

    #[test]
    #[expect(
        clippy::mem_forget,
        reason = "this test checks leaked MoveRef detection"
    )]
    #[should_panic(expected = "a critical reference counter at")]
    fn forget_move_ref_temporary()
    {
        core::mem::forget(expr!(&move 42_i32));
    }

    #[cfg_attr(miri, ignore)]
    #[cfg(all(feature = "alloc", not(feature = "valgrind")))]
    #[test]
    #[should_panic(expected = "a critical reference counter at")]
    fn forget_deref_moved_box()
    {
        let mut x = crate::Box::new(5);
        let ptr = x.as_mut() as *mut i32;
        core::mem::forget(expr!(&move *x));
        unsafe {
            alloc::alloc::dealloc(ptr as *mut u8, alloc::alloc::Layout::new::<i32>());
        }
    }

    #[test]
    fn release_inhibits_drop()
    {
        struct T;
        impl Drop for T
        {
            fn drop(&mut self)
            {
                panic!();
            }
        }
        let val = T;
        bind!(t = crate::new::of(val));
        let _released = MoveRef::release(t);
    }

    mod coverage
    {
        use super::*;

        mod move_ref
        {
            use super::*;

            const VAL1: &str = "value1";
            const VAL2: &str = "value2";

            #[test]
            fn as_ptr()
            {
                bind!(val = &move *Box::new(VAL1));
                let ptr = val.as_ptr();
                // SAFETY: ptr came from the live unique MoveRef and is still in its slot.
                assert_eq!(VAL1, unsafe { *ptr });
            }

            #[test]
            fn as_mut_ptr()
            {
                bind!(mut val = &move *Box::new(VAL1));
                let ptr = val.as_mut_ptr();
                // SAFETY: ptr came from the live unique MoveRef and is still in its slot.
                assert_eq!(VAL1, unsafe { *ptr });
                // SAFETY: ptr came from the live unique MoveRef and is still in its slot.
                unsafe {
                    ptr.write(VAL2);
                }
                // SAFETY: ptr came from the live unique MoveRef and is still in its slot.
                assert_eq!(VAL2, unsafe { *ptr });
            }

            #[test]
            fn deref_mut()
            {
                bind!(mut val = &move VAL1);
                assert_eq!(VAL1, *val);
                *val = VAL2;
                assert_eq!(VAL2, *val);
            }

            #[test]
            fn fmt()
            {
                use crate::alloc::format;
                bind!(val = &move VAL1);
                assert_eq!(format!("{VAL1:#?}"), format!("{val:#?}"));
            }

            #[test]
            fn partial_eq()
            {
                bind!(lhs = &move VAL1);
                bind!(rhs = &move VAL1);
                assert!(lhs.eq(&rhs));
            }

            #[test]
            fn partial_cmp()
            {
                bind!(lhs = &move VAL1);
                bind!(rhs = &move VAL1);
                assert!(matches!(
                    lhs.partial_cmp(&rhs),
                    Some(core::cmp::Ordering::Equal)
                ));
            }

            #[test]
            fn lt()
            {
                bind!(lhs = &move VAL1);
                bind!(rhs = &move VAL2);
                assert!(lhs.lt(&rhs));
            }

            #[test]
            fn le()
            {
                bind!(lhs = &move VAL1);
                bind!(rhs = &move VAL2);
                assert!(lhs.le(&rhs));
            }

            #[test]
            fn gt()
            {
                bind!(lhs = &move VAL2);
                bind!(rhs = &move VAL1);
                assert!(lhs.gt(&rhs));
            }

            #[test]
            fn ge()
            {
                bind!(lhs = &move VAL2);
                bind!(rhs = &move VAL1);
                assert!(lhs.ge(&rhs));
            }

            #[test]
            fn cmp()
            {
                bind!(lhs = &move VAL1);
                bind!(rhs = &move VAL2);
                assert!(matches!(lhs.cmp(&rhs), core::cmp::Ordering::Less));
            }

            #[cfg(feature = "default")]
            #[test]
            fn hash()
            {
                use core::hash::Hash as _;
                use core::hash::Hasher as _;
                bind!(lhs = &move VAL1);
                let hash1 = {
                    let mut hasher = seahash::SeaHasher::new();
                    lhs.hash(&mut hasher);
                    hasher.finish()
                };
                bind!(rhs = &move VAL1);
                let hash2 = {
                    let mut hasher = seahash::SeaHasher::new();
                    rhs.hash(&mut hasher);
                    hasher.finish()
                };
                assert_eq!(hash1, hash2);
            }
        }
    }
}
