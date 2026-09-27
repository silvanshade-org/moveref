#[cfg(feature = "alloc")]
use core::mem::MaybeUninit;
use core::ops::Deref;
#[cfg(feature = "alloc")]
use core::pin::Pin;

use crate::new::New;
use crate::new::TryNew;

/// Operations for constructing [`New`] values into a `Self::Output` instance.
///
/// # Specification
/// - requires: implementers allocate fresh stable storage for a `T`.
/// - ensures: construction yields an owner for the initialized referent.
/// - provides: pinned placement for infallible or fallible initializers.
pub trait Emplace<T>: Sized + Deref
{
    /// Pinned output ownership of the initialized referent.
    type Output: Deref<Target = Self::Target>;

    /// Construct a [`New`] value into a fresh `Self::Output` instance.
    ///
    /// # Specification
    /// - requires: `new` initializes fresh pinned storage.
    /// - ensures: returns a pinned owning value initialized by `new`.
    /// - provides: an owning allocation containing `T`.
    /// - panics: follows allocation and initializer operations.
    ///
    /// # Adequacy
    /// - hypothesis: a failed destructor path would change the observed drop
    ///   count.
    /// - witness: `emplace::tests::box_emplace_drops_once`.
    #[inline]
    fn emplace<N>(new: N) -> Self::Output
    where
        N: New<Output = T>,
    {
        match Self::try_emplace(new) {
            | Ok(val) => return val,
            | Err(err) => match err {},
        }
    }

    /// Try to construct a [`New`] value into a fresh `Self::Output` instance.
    ///
    /// # Specification
    /// - requires: `new` may initialize fresh pinned storage.
    /// - ensures: on success, returns a pinned owning value.
    /// - provides: the initialized output or the initializer’s error.
    /// - fails: propagates `N::Error` from the initializer.
    /// - panics: follows allocation and initializer operations.
    ///
    /// # Adequacy
    /// - hypothesis: replacing the initializer’s error loses its identity.
    /// - witness: `emplace::tests::fallible_heap_emplacement_propagates_exact_error`.
    ///
    /// # Errors
    ///
    /// Should return `Err` if the `new` initializer fails with an error.
    fn try_emplace<N>(new: N) -> Result<Self::Output, N::Error>
    where
        N: TryNew<Output = T>;
}

#[cfg(feature = "alloc")]
impl<T> Emplace<T> for crate::Box<T>
{
    type Output = Pin<Self>;

    #[inline]
    /// Attempt placement into a fresh boxed allocation.
    ///
    /// # Specification
    /// - requires: `new` satisfies the pinned placement initialization
    ///   contract.
    /// - ensures: on success, returns a pinned `Box<T>` at its initialized
    ///   allocation.
    /// - provides: ownership of the initialized heap allocation.
    /// - fails: propagates the initializer’s exact error on failure.
    /// - panics: follows the initializer and allocation operations.
    ///
    /// # Errors
    /// - `N::Error`: the initializer returned an error.
    ///
    /// # Adequacy
    /// - hypothesis: a successful placement drops once; a failed placement
    ///   preserves its error.
    /// - witness: `emplace::tests::box_emplace_drops_once`.
    /// - witness: `emplace::tests::fallible_heap_emplacement_propagates_exact_error`.
    fn try_emplace<N>(new: N) -> Result<Self::Output, N::Error>
    where
        N: TryNew<Output = T>,
    {
        let mut uninit = crate::Box::new(MaybeUninit::<T>::uninit());
        // SAFETY: this fresh allocation cannot move while the initializer holds its
        // pin.
        let pin = unsafe { Pin::new_unchecked(&mut *uninit) };
        // SAFETY: the initializer receives fresh pinned uninitialized storage.
        unsafe { new.try_new(pin)? };
        // SAFETY: successful initialization left a valid T in the same allocation.
        let ptr = unsafe { Self::from_raw(crate::Box::into_raw(uninit).cast::<T>()) };
        return Ok(Self::into_pin(ptr));
    }
}

#[cfg(feature = "alloc")]
impl<T> Emplace<T> for crate::Rc<T>
{
    type Output = Pin<Self>;

    #[inline]
    /// Attempt placement into a fresh reference-counted allocation.
    ///
    /// # Specification
    /// - requires: `new` satisfies the pinned placement initialization
    ///   contract.
    /// - ensures: on success, returns a pinned `Rc<T>` at its initialized
    ///   allocation.
    /// - provides: unique construction before any shared references escape.
    /// - fails: propagates the initializer’s exact error on failure.
    /// - panics: follows the initializer and allocation operations.
    ///
    /// # Errors
    /// - `N::Error`: the initializer returned an error.
    ///
    /// # Adequacy
    /// - hypothesis: a successful placement drops once; a failed placement
    ///   preserves its error.
    /// - witness: `emplace::tests::rc_emplace_drops_once`.
    /// - witness: `emplace::tests::fallible_heap_emplacement_propagates_exact_error`.
    fn try_emplace<N>(new: N) -> Result<Self::Output, N::Error>
    where
        N: TryNew<Output = T>,
    {
        let mut uninit = crate::Rc::new(MaybeUninit::<T>::uninit());
        #[expect(
            clippy::expect_used,
            reason = "a freshly allocated Rc has one strong owner"
        )]
        let ptr = crate::Rc::get_mut(&mut uninit).expect("fresh Rc is unique");
        // SAFETY: this fresh allocation cannot move while the initializer holds its
        // pin.
        let pin = unsafe { Pin::new_unchecked(ptr) };
        // SAFETY: the initializer receives fresh pinned uninitialized storage.
        unsafe { new.try_new(pin)? };
        // SAFETY: successful initialization left a valid T in the same allocation.
        let ptr = unsafe { Self::from_raw(crate::Rc::into_raw(uninit).cast::<T>()) };
        // SAFETY: the Rc points to the initialized T and its allocation stays stable.
        let pin = unsafe { Pin::new_unchecked(ptr) };
        return Ok(pin);
    }
}

#[cfg(feature = "alloc")]
impl<T> Emplace<T> for crate::Arc<T>
{
    type Output = Pin<Self>;

    #[inline]
    /// Attempt placement into a fresh shared allocation.
    ///
    /// # Specification
    /// - requires: `new` satisfies the pinned placement initialization
    ///   contract.
    /// - ensures: on success, returns a pinned `Arc<T>` at its initialized
    ///   allocation.
    /// - provides: unique construction before any shared references escape.
    /// - fails: propagates the initializer’s exact error on failure.
    /// - panics: follows the initializer and allocation operations.
    ///
    /// # Errors
    /// - `N::Error`: the initializer returned an error.
    ///
    /// # Adequacy
    /// - hypothesis: a successful placement drops once; a failed placement
    ///   preserves its error.
    /// - witness: `emplace::tests::arc_emplace_drops_once`.
    /// - witness: `emplace::tests::fallible_heap_emplacement_propagates_exact_error`.
    fn try_emplace<N>(new: N) -> Result<Self::Output, N::Error>
    where
        N: TryNew<Output = T>,
    {
        let mut uninit = crate::Arc::new(MaybeUninit::<T>::uninit());
        #[expect(
            clippy::expect_used,
            reason = "a freshly allocated Arc has one strong owner"
        )]
        let ptr = crate::Arc::get_mut(&mut uninit).expect("fresh Arc is unique");
        // SAFETY: this fresh allocation cannot move while the initializer holds its
        // pin.
        let pin = unsafe { Pin::new_unchecked(ptr) };
        // SAFETY: the initializer receives fresh pinned uninitialized storage.
        unsafe { new.try_new(pin)? };
        // SAFETY: successful initialization left a valid T in the same allocation.
        let ptr = unsafe { Self::from_raw(crate::Arc::into_raw(uninit).cast::<T>()) };
        // SAFETY: the Arc points to the initialized T and its allocation stays stable.
        let pin = unsafe { Pin::new_unchecked(ptr) };
        return Ok(pin);
    }
}

#[cfg(test)]
mod tests
{
    #[cfg(feature = "alloc")]
    use core::cell::Cell;

    #[cfg(feature = "alloc")]
    #[repr(transparent)]
    struct Counted<'counter>(&'counter Cell<usize>);

    #[cfg(feature = "alloc")]
    impl Drop for Counted<'_>
    {
        fn drop(&mut self)
        {
            self.0.set(self.0.get().saturating_add(1));
        }
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn box_emplace_drops_once()
    {
        let drops = Cell::new(0);
        let pinned = <crate::Box<_> as crate::Emplace<_>>::emplace(crate::new::of(Counted(&drops)));
        assert_eq!(drops.get(), 0);
        drop(pinned);
        assert_eq!(drops.get(), 1);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn rc_emplace_drops_once()
    {
        let drops = Cell::new(0);
        let pinned = <crate::Rc<_> as crate::Emplace<_>>::emplace(crate::new::of(Counted(&drops)));
        assert_eq!(drops.get(), 0);
        drop(pinned);
        assert_eq!(drops.get(), 1);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn arc_emplace_drops_once()
    {
        let drops = Cell::new(0);
        let pinned = <crate::Arc<_> as crate::Emplace<_>>::emplace(crate::new::of(Counted(&drops)));
        assert_eq!(drops.get(), 0);
        drop(pinned);
        assert_eq!(drops.get(), 1);
    }
    #[cfg(feature = "alloc")]
    #[derive(Debug, PartialEq)]
    struct Rejected;

    #[cfg(feature = "alloc")]
    struct Reject;

    #[cfg(feature = "alloc")]
    impl crate::new::TryNew for Reject
    {
        type Output = u8;
        type Error = Rejected;

        /// Fail after producing an initialized placement value.
        ///
        /// # Specification
        /// - ensures: leaves a valid `u8` in the supplied storage.
        /// - fails: always returns `Rejected`.
        /// - panics: none.
        /// - unsafe invariants: writes only to fresh pinned storage.
        ///
        /// # Safety
        /// The caller supplies fresh pinned storage.
        unsafe fn try_new(
            self,
            mut this: core::pin::Pin<&mut core::mem::MaybeUninit<u8>>,
        ) -> Result<(), Self::Error>
        {
            this.set(core::mem::MaybeUninit::new(17_u8));
            Err(Rejected)
        }
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn fallible_heap_emplacement_propagates_exact_error()
    {
        let boxed = <crate::Box<u8> as crate::Emplace<u8>>::try_emplace(Reject);
        let shared = <crate::Rc<u8> as crate::Emplace<u8>>::try_emplace(Reject);
        let atomic = <crate::Arc<u8> as crate::Emplace<u8>>::try_emplace(Reject);
        assert!(matches!(boxed, Err(Rejected)));
        assert!(matches!(shared, Err(Rejected)));
        assert!(matches!(atomic, Err(Rejected)));
    }

    mod coverage
    {
        mod emplace
        {
            #[cfg(feature = "alloc")]
            #[test]
            fn arc()
            {
                const VAL: u8 = 128;
                let new = crate::new::by(move || return VAL);
                let out = <crate::Arc<_> as crate::Emplace<_>>::emplace(new);
                assert_eq!(VAL, *out);
            }

            #[cfg(feature = "alloc")]
            #[test]
            fn r#box()
            {
                const VAL: u8 = 128;
                let new = crate::new::by(move || return VAL);
                let out = <crate::Box<_> as crate::Emplace<_>>::emplace(new);
                assert_eq!(VAL, *out);
            }

            #[cfg(feature = "alloc")]
            #[test]
            fn rc()
            {
                const VAL: u8 = 128;
                let new = crate::new::by(move || return VAL);
                let out = <crate::Rc<_> as crate::Emplace<_>>::emplace(new);
                assert_eq!(VAL, *out);
            }
        }
    }
}
