use core::cell::Cell;
use core::mem::MaybeUninit;

use crate::slot::Slot;

/// State for tracking the status of a storage [`Slot`].
///
/// # Specification
/// - ensures: tracks initialization, release, and exactly one outstanding
///   owner.
/// - provides: shared state to a slot and its owning move-reference.
pub struct SlotStorageTracker
{
    /// Whether the [`Slot`] is initialized.
    initialized: Cell<bool>,
    /// Whether the [`Slot`] is released. If released, [`Drop`] will be skipped.
    released: Cell<bool>,
    /// Number of references to the [`Slot`]. Used for checking various
    /// conditions.
    references: Cell<usize>,
}

impl SlotStorageTracker
{
    /// Construct a new [`SlotStorageTracker`].
    ///
    /// # Specification
    /// - ensures: initializes all status flags to false and reference count to
    ///   zero.
    /// - provides: an empty tracker.
    /// - panics: none.
    #[inline]
    pub const fn new() -> Self
    {
        return Self {
            initialized: Cell::new(false),
            released: Cell::new(false),
            references: Cell::new(0),
        };
    }

    /// Project the the status by borrowing the internal state.
    ///
    /// # Specification
    /// - ensures: returns handles to this tracker’s state cells.
    /// - provides: a borrowed status for slot operations.
    /// - panics: none.
    #[inline]
    pub const fn status(&self) -> SlotStorageStatus<'_>
    {
        return SlotStorageStatus {
            initialized: &self.initialized,
            released: &self.released,
            references: &self.references,
        };
    }
}

/// The borrowed form of [`SlotStorageTracker`].
///
/// # Specification
/// - ensures: borrows the tracker’s mutable state for the frame lifetime.
/// - provides: copyable status handles that share the same state cells.
#[derive(Clone, Copy)]
pub struct SlotStorageStatus<'frame>
{
    /// Whether the [`Slot`] is initialized.
    initialized: &'frame Cell<bool>,
    /// Whether the [`Slot`] is released. If released, [`Drop`] will be skipped.
    released: &'frame Cell<bool>,
    /// Number of references to the [`Slot`]. Used for checking various
    /// conditions.
    references: &'frame Cell<usize>,
}

impl SlotStorageStatus<'_>
{
    /// Set the status to initialized.
    ///
    /// # Specification
    /// - requires: a fresh slot with no initialized value.
    /// - ensures: marks the slot initialized and records one owner.
    /// - panics: a debug build panics if it was already initialized.
    #[inline]
    pub(crate) fn initialize(&self)
    {
        debug_assert!(!self.is_initialized(), "slot initialized twice");
        self.initialized.set(true);
        self.increment();
    }

    /// Increment the reference count.
    ///
    /// # Specification
    /// - requires: no outstanding owner.
    /// - ensures: the tracked reference count becomes one.
    /// - panics: a debug build panics if an owner already exists.
    #[inline]
    pub(crate) fn increment(&self)
    {
        debug_assert!(
            self.is_reference_zeroed(),
            "increment requires an empty slot"
        );
        self.references.set(1);
    }

    /// Decrement the reference count.
    ///
    /// # Specification
    /// - requires: exactly one outstanding reference.
    /// - ensures: the tracked reference count becomes zero.
    /// - panics: a debug build panics if the count is not one.
    #[inline]
    pub(crate) fn decrement(&self)
    {
        debug_assert_eq!(self.references.get(), 1, "decrement requires one owner");
        self.references.set(0);
    }

    /// Mark the storage as released. Once marked as released, [`Drop`] will be
    /// skipped.
    ///
    /// # Specification
    /// - requires: the caller has transferred the referent’s destruction
    ///   obligation.
    /// - ensures: the tracker no longer asks a move-reference to drop the
    ///   referent.
    /// - panics: none.
    /// - unsafe invariants: the referent is still destroyed by its new owner,
    ///   or intentionally leaked.
    ///
    /// # Adequacy
    /// - hypothesis: retaining the old destructor would run it twice after
    ///   release.
    /// - witness: `slot::tests::release_never_implicitly_drops_the_referent`.
    ///
    /// # Safety
    ///
    /// The caller must transfer responsibility for destroying the initialized
    /// referent before marking this slot released.
    #[inline]
    pub(crate) unsafe fn release(&self)
    {
        self.released.set(true);
    }

    /// Mark the storage as terminated. This is just a decrement followed by an
    /// assertion that references are finally zeroed. It is intended to be
    /// called only when the storage is dropped.
    ///
    /// # Specification
    /// - requires: one outstanding owner is terminating.
    /// - ensures: the reference count reaches zero.
    /// - panics: a debug build panics if no owner exists or a reference
    ///   remains.
    #[inline]
    pub(crate) fn terminate(&self)
    {
        self.decrement();
        debug_assert!(
            self.is_reference_zeroed(),
            "terminated storage still has references"
        );
    }

    /// Check if the storage is initialized.
    ///
    /// # Specification
    /// trivial.
    #[inline]
    pub(crate) fn is_initialized(&self) -> bool
    {
        return self.initialized.get();
    }

    /// Check if the storage is uninitialized.
    ///
    /// # Specification
    /// - ensures: true exactly when no value was initialized and no owner
    ///   remains.
    /// - provides: the empty-slot predicate.
    /// - panics: none.
    #[inline]
    pub(crate) fn is_uninitialized(&self) -> bool
    {
        return self.is_reference_zeroed() && !self.is_initialized();
    }

    /// Check if the storage is released.
    ///
    /// # Specification
    /// trivial.
    #[inline]
    pub(crate) fn is_released(&self) -> bool
    {
        return self.released.get();
    }

    /// Check if the storage is leaking.
    ///
    /// # Specification
    /// - ensures: true exactly when initialized and owned but not released.
    /// - provides: the leak-detection predicate.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: a forgotten owner leaves one outstanding reference.
    /// - witness: `move_ref::test::forget_move_ref`.
    #[inline]
    pub(crate) fn is_leaking(&self) -> bool
    {
        return !self.is_released() && self.is_initialized() && !self.is_reference_zeroed();
    }

    /// Check if the references are zeroed.
    ///
    /// # Specification
    /// trivial.
    #[inline]
    pub(crate) fn is_reference_zeroed(&self) -> bool
    {
        return self.references.get() == 0;
    }
}

/// Policy for dropping the stored `T` after its owning move-reference ends.
///
/// # Specification
/// - ensures: `Keep` skips the storage destructor; `Drop` runs it after the
///   reference ends.
/// - provides: explicit ownership of the stored value or of a separate backing
///   allocation.
///
/// `Drop` is for independent resources such as `Box<MaybeUninit<U>>`, not a
/// directly stored referent that `MoveRef` already destroys.
#[derive(Copy, Clone, Debug)]
pub enum SlotStorageKind
{
    /// Destroy the stored resource when storage leaves scope.
    ///
    /// # Specification
    /// - requires: `T` remains live after the move-reference destroys its
    ///   referent.
    /// - ensures: invokes the stored `T` destructor.
    Drop,
    /// Leave the stored memory untouched when storage leaves scope.
    ///
    /// # Specification
    /// - ensures: no destructor runs from storage for `T`.
    Keep,
}

/// Type used for constructing the storage for a [`Slot`] backing a
/// [`MoveRef`](crate::MoveRef).
///
/// # Specification
/// - requires: `Drop` is chosen only when the stored `T` remains initialized
///   after the owning move-reference ends.
/// - ensures: the policy controls destruction of stored `T`, not necessarily
///   its referent.
/// - provides: backing memory and an ownership tracker for a move-reference.
///
/// # Known defect
/// The safe `Drop` policy also accepts a directly stored referent and destroys
/// it twice; use `Keep` for direct values.
pub struct SlotStorage<T>
{
    /// The kind dictating the storage drop behavior.
    kind: SlotStorageKind,
    /// The raw underlying (possibly uninitialized) storage memory.
    memory: MaybeUninit<T>,
    /// Status flags for the storage which track initialization, dropping state,
    /// and reference count.
    tracker: SlotStorageTracker,
    /// Location for reporting panic data.
    #[cfg(debug_assertions)]
    location: &'static core::panic::Location<'static>,
}

impl<T> Drop for SlotStorage<T>
{
    /// Check the ownership tracker and dispose of the stored value according to
    /// its policy.
    ///
    /// # Specification
    /// - ensures: empty storage is inert; a leaked owner aborts; valid `Drop`
    ///   storage drops its stored `T`.
    /// - provides: no stored-value destructor under `Keep`.
    /// - panics: leaked-owner detection aborts outside unit tests.
    ///
    /// # Adequacy
    /// - hypothesis: failure to abort would let a forgotten owner outlive
    ///   storage.
    /// - witness: `tests/leak_abort.
    ///   rs::forgetting_a_move_ref_aborts_the_process`.
    #[inline]
    fn drop(&mut self)
    {
        let status = self.tracker.status();
        if status.is_uninitialized() {
            // NOTE: the only time this should happen is when the `SlotStorage` is created
            // manually, outside the use of the macros, since otherwise the
            // storage is initialized immediately after creation.
            return;
        }
        if status.is_leaking() {
            self.non_unwinding_panic_abort();
        }
        if matches!(self.kind, SlotStorageKind::Drop) {
            // SAFETY: valid only when `Drop` storage owns a separate live resource such as
            // Box<MaybeUninit<U>>. Directly stored referents violate this assumption;
            // the ignored Miri regression records the current public-API defect.
            unsafe { self.memory.assume_init_drop() }
        }
    }
}

impl<T> SlotStorage<T>
{
    /// Construct a new [`SlotStorage<T>`] given a `kind`.
    ///
    /// # Specification
    /// - requires: choose `Keep` for directly stored referents; `Drop` only if
    ///   the stored `T` remains valid after the move-reference destructor.
    /// - ensures: starts with uninitialized memory and no outstanding
    ///   reference.
    /// - provides: backing storage governed by `kind`.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: `Drop` on a directly owned value violates single
    ///   destruction.
    /// - witness: `slot::tests::dropping_direct_value_with_drop_kind_must_not_destroy_twice` (ignored Miri failure).
    #[must_use]
    #[inline]
    pub const fn new(kind: SlotStorageKind) -> Self
    {
        return Self {
            kind,
            memory: MaybeUninit::uninit(),
            tracker: SlotStorageTracker::new(),
            #[cfg(debug_assertions)]
            location: core::panic::Location::caller(),
        };
    }

    /// Project the [`Slot`] for the storage.
    ///
    /// # Specification
    /// - requires: no slot view currently holds this mutable storage borrow.
    /// - ensures: lends fresh placement memory and the matching tracker
    ///   together.
    /// - provides: a slot bounded by this storage borrow.
    /// - panics: none.
    #[inline]
    pub fn slot(&mut self) -> Slot<'_, T>
    {
        let memory = &mut self.memory;
        let status = self.tracker.status();
        return Slot::new(memory, status);
    }

    /// Report the source location for a leaked reference.
    ///
    /// # Specification
    /// - ensures: exposes the recorded creation location in debug builds or an
    ///   unknown marker otherwise.
    /// - provides: displayable diagnostic context for a leaked owner.
    /// - panics: none.
    #[inline]
    pub fn display_location(&self) -> &dyn core::fmt::Display
    {
        /// Placeholder location display.
        const UNKNOWN: &str = "<unknown>";

        if cfg!(debug_assertions) {
            return self.location;
        }
        return &UNKNOWN;
    }

    /// Abort when a storage slot still has an outstanding owner at destruction.
    ///
    /// # Specification
    /// - requires: a leaked owner remains tracked when storage leaves scope.
    /// - ensures: production process aborts rather than continuing with invalid
    ///   storage.
    /// - panics: unit tests panic once; production aborts during nested panics.
    ///
    /// # Adequacy
    /// - hypothesis: an ordinary panic would not terminate the subprocess by
    ///   SIGABRT.
    /// - witness: `tests/leak_abort.
    ///   rs::forgetting_a_move_ref_aborts_the_process`.
    #[expect(
        clippy::panic,
        reason = "abort on leaked ownership is the documented invariant"
    )]
    fn non_unwinding_panic_abort(&self)
    {
        /// Guard whose destructor triggers a second panic outside unit tests.
        struct DropAndPanic;

        impl Drop for DropAndPanic
        {
            /// Trigger the second panic when this guard is dropped during
            /// unwinding.
            ///
            /// # Specification
            /// - ensures: production builds abort through nested panics; tests
            ///   retain the first panic.
            /// - panics: intentionally in production builds.
            #[inline]
            fn drop(&mut self)
            {
                #[expect(
                    clippy::manual_assert,
                    reason = "the nested panic forces an abort outside tests"
                )]
                #[expect(clippy::panic, reason = "intentionally aborts on leaked ownership")]
                if cfg!(not(test)) {
                    panic!("initiating double-panic to trigger an LLVM abort")
                }
            }
        }

        // Trigger the first panic.
        let _first_panic_trigger = DropAndPanic;

        // Trigger the second panic mid-unwind.
        panic!(
            "a critical reference counter at {} was not zeroed!",
            self.display_location()
        );
    }
}
