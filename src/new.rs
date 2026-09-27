use core::mem::MaybeUninit;
use core::pin::Pin;

use crate::into_move::IntoMove;
use crate::move_ref::MoveRef;

/// Types which can be constructed (initialized) into some provided storage.
///
/// # Specification
/// - ensures: implementations initialize a valid output in supplied pinned
///   storage.
/// - provides: infallible placement construction.
pub trait New: Sized
{
    type Output;

    /// Initialize `Self` using `this` for storage.
    ///
    /// # Specification
    /// - requires: `this` points to fresh pinned storage.
    /// - ensures: initializes `this` with one valid `Self::Output`.
    /// - panics: follows the initializer implementation.
    /// - unsafe invariants: no initialized object is overwritten or left
    ///   invalid on return.
    ///
    /// # Safety
    ///
    /// - [`New::new()`] must not be used to mutate previously initialized data
    /// - `this` must be freshly-allocated memory
    /// - after invocation, the `this` placement argument is in a valid,
    ///   initialized state
    #[expect(
        clippy::new_ret_no_self,
        clippy::wrong_self_convention,
        reason = "New::new constructs its Output in supplied storage"
    )]
    unsafe fn new(
        self,
        this: Pin<&mut MaybeUninit<Self::Output>>,
    );
}

/// Types which can be constructed (initialized) into some provided storage.
/// Construction may fail.
///
/// # Specification
/// - ensures: implementations report initialization failures through `Error`.
/// - provides: fallible placement construction into caller-supplied storage.
#[expect(
    clippy::module_name_repetitions,
    reason = "TryNew distinguishes fallible construction"
)]
pub trait TryNew
{
    type Output;
    type Error;

    /// Try to initialize `Self` using `this` for storage.
    ///
    /// # Specification
    /// - requires: `this` points to fresh pinned storage.
    /// - ensures: on success, `this` holds a valid initialized output.
    /// - provides: a success result or the initializer’s error.
    /// - fails: returns `Self::Error` if initialization cannot complete.
    /// - panics: follows the initializer implementation.
    /// - unsafe invariants: the implementer must leave `this` valid after
    ///   returning, including on error.
    ///
    /// # Errors
    ///
    /// Should return `Err` if initialization failed.
    ///
    /// # Safety
    ///
    /// - [`TryNew::try_new()`] must not be used to mutate previously
    ///   initialized data
    /// - `this` must be freshly-allocated memory
    /// - after invocation, the `this` placement argument is in a valid,
    ///   initialized state
    unsafe fn try_new(
        self,
        this: Pin<&mut MaybeUninit<Self::Output>>,
    ) -> Result<(), Self::Error>;
}

impl<N: New> TryNew for N
{
    type Output = N::Output;
    type Error = core::convert::Infallible;

    #[inline]
    unsafe fn try_new(
        self,
        this: Pin<&mut MaybeUninit<Self::Output>>,
    ) -> Result<(), Self::Error>
    {
        // SAFETY: `TryNew` inherits the caller's `New` initialization requirements.
        unsafe { self.new(this) };
        return Ok(());
    }
}

/// Types which can be copy-constructed from an existing value into some
/// provided storage.
///
/// # Specification
/// - ensures: implementations copy values into fresh pinned destination
///   storage.
/// - provides: copy construction without consuming the source.
#[expect(
    clippy::module_name_repetitions,
    reason = "CopyNew names the constructor contract"
)]
pub trait CopyNew: Sized
{
    /// Copy-construct a value into fresh pinned storage.
    ///
    /// # Specification
    /// - requires: `src` is initialized and `dst` is fresh pinned storage.
    /// - ensures: leaves `src` unchanged and initializes `dst` with an
    ///   equivalent value.
    /// - panics: follows the implementer’s copy operation.
    /// - unsafe invariants: `dst` holds one valid `Self` after successful
    ///   return.
    ///
    /// # Safety
    /// The caller supplies fresh pinned storage and a valid source.
    unsafe fn copy_new(
        src: &Self,
        dst: Pin<&mut MaybeUninit<Self>>,
    );
}

/// Types which can be move-constructed from an existing value into some
/// provided storage.
///
/// # Specification
/// - ensures: implementations transfer unique ownership into pinned destination
///   storage.
/// - provides: a move-construction operation without implicit relocation of the
///   referent.
#[expect(
    clippy::module_name_repetitions,
    reason = "MoveNew names the constructor contract"
)]
pub trait MoveNew: Sized
{
    /// Move-construct a value into fresh pinned storage.
    ///
    /// # Specification
    /// - requires: `src` uniquely owns an initialized referent; `dst` is fresh
    ///   pinned storage.
    /// - ensures: consumes the source and initializes the destination once.
    /// - panics: follows the implementer’s move operation.
    /// - unsafe invariants: no source referent is dropped twice or left owned
    ///   without a destructor.
    ///
    /// # Safety
    /// The caller supplies an owning pinned source and fresh pinned storage.
    unsafe fn move_new(
        src: Pin<MoveRef<'_, Self>>,
        dst: Pin<&mut MaybeUninit<Self>>,
    );
}

/// Constructs a [`New`] value using a thunk which initializes its data into
/// some pinned, uninitialized memory.
///
/// # Specification
/// - requires: `initializer` initializes fresh pinned storage exactly once.
/// - ensures: calling the returned initializer writes a valid `T`.
/// - provides: an owning deferred initializer.
/// - panics: propagates a panic from `initializer`.
/// - unsafe invariants: the thunk leaves initialized storage after successful
///   return.
///
/// # Adequacy
/// - hypothesis: failing to write into the pinned destination would be observed
///   by subsequent use.
/// - witness: `emplace::tests::coverage::emplace::r#box`.
///
/// # Safety
///
/// - `initializer` must satisfy the same safety requirements as [`New::new()`]
#[inline]
pub unsafe fn by_raw<T, F>(initializer: F) -> impl New<Output = T>
where
    F: FnOnce(Pin<&mut MaybeUninit<T>>),
{
    /// Helper type for converting into the abstract `impl New`.
    struct FnNew<F, T>
    {
        /// The underlying thunk.
        initializer: F,
        /// Phantom type holding `T`, respecting variance.
        _type: core::marker::PhantomData<fn(Pin<&mut MaybeUninit<T>>)>,
    }

    impl<F, T> New for FnNew<F, T>
    where
        F: FnOnce(Pin<&mut MaybeUninit<T>>),
    {
        type Output = T;
        /// Invoke the raw initializer with the supplied pinned storage.
        ///
        /// # Specification
        /// - requires: the storage is fresh and pinned.
        /// - ensures: invokes the initializer once with that storage.
        /// - panics: propagates a panic from the initializer.
        /// - unsafe invariants: the initializer must leave valid `T` after
        ///   return.
        ///
        /// # Safety
        /// The caller supplies fresh pinned storage; the initializer must write
        /// a valid `T`.
        #[inline]
        unsafe fn new(
            self,
            this: Pin<&mut MaybeUninit<Self::Output>>,
        )
        {
            (self.initializer)(this);
        }
    }

    return FnNew {
        initializer,
        _type: core::marker::PhantomData,
    };
}

/// Constructs a [`New`] value using a value-producing thunk `f`.
///
/// # Specification
/// - requires: `f` returns a valid `T`.
/// - ensures: calls `f` during `by`, then places the resulting `T` when
///   initialized.
/// - provides: a deferred constructor owning the produced value.
/// - panics: propagates a panic from `f`.
///
/// # Adequacy
/// - hypothesis: calling `f` at initialization instead of at constructor
///   creation is observably different.
/// - witness: `new::test::by_evaluates_thunk_before_placement`.
#[inline]
pub fn by<T, F>(f: F) -> impl New<Output = T>
where
    F: FnOnce() -> T,
{
    let val = f();
    // SAFETY: the initializer writes exactly one valid value into fresh pinned
    // storage.
    unsafe { return by_raw(|mut dst| dst.set(MaybeUninit::new(val))) }
}

/// Constructs a [`New`] value using a given value `val`.
///
/// # Specification
/// - ensures: a later initialization places exactly `val` in the supplied
///   storage.
/// - provides: a constructor owning `val` until use.
/// - panics: none.
#[inline]
pub fn of<T>(val: T) -> impl New<Output = T>
{
    return by(|| return val);
}

/// Constructs a [`New`] value for a type `T` using it's default value.
///
/// # Specification
/// - ensures: calls `T::default` when constructing the initializer and places
///   that value on use.
/// - provides: a constructor for the default value.
/// - panics: propagates a panic from `T::default`.
#[inline]
pub fn default<T>() -> impl New<Output = T>
where
    T: Default,
{
    return by(Default::default);
}

/// Construct a [`New`] value by consuming a uniquely owning pointer into the
/// provided storage.
///
/// # Specification
/// - requires: `ptr` uniquely owns its referent and `P::Target` implements a
///   sound move constructor.
/// - ensures: invocation transfers the referent into the supplied pinned
///   storage.
/// - provides: a deferred constructor consuming `ptr` at initialization.
/// - panics: propagates a panic from the move constructor.
///
/// # Adequacy
/// - hypothesis: consumption without destruction differs from either dropping
///   source early or never dropping destination.
/// - witness: `new::test::mov_transfers_referent_without_premature_destruction`.
#[inline]
pub fn mov<P>(ptr: P) -> impl New<Output = P::Target>
where
    P: IntoMove,
    P::Target: MoveNew,
{
    let initializer = move |dst: Pin<&mut MaybeUninit<P::Target>>| {
        bind_slot!(
            #[dropping]
            storage
        );
        let src = ptr.into_move(storage);
        // SAFETY: src is an owning pinned reference, dst is fresh pinned storage.
        unsafe { MoveNew::move_new(src, dst) };
    };
    // SAFETY: MoveNew writes the destination initialized from the consumed unique
    // source.
    return unsafe { by_raw(initializer) };
}

#[cfg(test)]
mod test
{
    use core::cell::Cell;
    use core::mem::MaybeUninit;
    use core::pin::Pin;

    use crate::MoveNew;
    use crate::MoveRef;
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

    impl MoveNew for Counted<'_>
    {
        /// Move an owned referent into a fresh pinned slot.
        ///
        /// # Specification
        /// - requires: src uniquely owns an initialized referent; dst is fresh
        ///   pinned storage.
        /// - ensures: transfers the value without running its destructor.
        /// - panics: none.
        /// - unsafe invariants: source ownership is consumed exactly once.
        ///
        /// # Safety
        /// The caller supplies a uniquely owned pinned source and fresh pinned
        /// destination.
        #[inline]
        unsafe fn move_new(
            src: Pin<MoveRef<'_, Self>>,
            mut dst: Pin<&mut MaybeUninit<Self>>,
        )
        {
            let value = MoveRef::into_inner(Pin::into_inner(src));
            dst.set(MaybeUninit::new(value));
        }
    }

    #[test]
    fn mov_transfers_referent_without_premature_destruction()
    {
        let drops = Cell::new(0);
        let mut source = SlotStorage::new(SlotStorageKind::Keep);
        let src = source.slot().put(Counted(&drops));
        let mut destination = SlotStorage::new(SlotStorageKind::Keep);
        let moved = destination.slot().emplace(super::mov(src));
        assert_eq!(drops.get(), 0);
        drop(moved);
        drop(destination);
        drop(source);
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn by_evaluates_thunk_before_placement()
    {
        let calls = Cell::new(0_usize);
        let initializer = super::by(|| {
            calls.set(calls.get().saturating_add(1));
            19_u8
        });
        assert_eq!(calls.get(), 1);
        let mut storage = SlotStorage::new(SlotStorageKind::Keep);
        let placed = storage.slot().emplace(initializer);
        assert_eq!(*placed, 19_u8);
        assert_eq!(calls.get(), 1);
        drop(placed);
    }

    #[test]
    fn default_places_the_type_default()
    {
        let mut storage = SlotStorage::new(SlotStorageKind::Keep);
        let placed = storage.slot().emplace(super::default::<u8>());
        assert_eq!(*placed, 0_u8);
        drop(placed);
    }
}
