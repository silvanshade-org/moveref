#![no_std]
#![expect(
    clippy::multiple_crate_versions,
    reason = "cxxbridge-macro and tracing-attributes depend on different syn majors"
)]

//! Types and traits for C++ style placement initialization and move semantics.

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
pub(crate) use alloc::boxed::Box;
#[cfg(feature = "alloc")]
pub(crate) use alloc::rc::Rc;
#[cfg(feature = "alloc")]
pub(crate) use alloc::sync::Arc;

/// Macros for creating [`crate::MoveRef`] values.
#[macro_use]
mod macros;

/// Dereferencing move operations.
mod deref_move;
/// Emplacement operations for constructing values.
mod emplace;
/// Movement operations.
mod into_move;
/// Move-dereferencing uniquely-owning references.
mod move_ref;
/// Construction operations.
pub mod new;
/// Storage slots for move-references.
mod slot;
/// Storage slot implementation details.
mod slot_storage;

pub use deref_move::DerefMove;
pub use emplace::Emplace;
pub use into_move::IntoMove;
pub use move_ref::MoveRef;
pub use new::CopyNew;
pub use new::MoveNew;
pub use new::New;
pub use slot::Slot;
pub use slot_storage::SlotStorage;
pub use slot_storage::SlotStorageKind;

trivial_copy! {
    (),
    bool,
    char,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,

    &T where [T: ?Sized],
    *const T where [T: ?Sized],

    *mut T where [T: ?Sized],

    ::core::alloc::Layout,

    ::core::cell::UnsafeCell<T> where [T],
    ::core::cell::Cell<T> where [T],
    ::core::cell::RefCell<T> where [T],
    ::core::cell::Ref<'_, T> where [T],
    ::core::cell::RefMut<'_, T> where [T],

    ::core::marker::PhantomData<T> where [T: ?Sized],
    ::core::marker::PhantomPinned,

    ::core::mem::Discriminant<T> where [T],
    ::core::mem::ManuallyDrop<T> where [T],
    ::core::mem::MaybeUninit<T> where [T],

    ::core::num::NonZeroI8,
    ::core::num::NonZeroI16,
    ::core::num::NonZeroI32,
    ::core::num::NonZeroI64,
    ::core::num::NonZeroI128,
    ::core::num::NonZeroIsize,
    ::core::num::NonZeroU8,
    ::core::num::NonZeroU16,
    ::core::num::NonZeroU32,
    ::core::num::NonZeroU64,
    ::core::num::NonZeroU128,
    ::core::num::NonZeroUsize,
    ::core::num::Wrapping<T> where [T],

    ::core::option::Option<T> where [T],

    ::core::pin::Pin<T> where [T],
    ::core::ptr::NonNull<T> where [T],

    ::core::result::Result<T, E> where [T, E],

    ::core::time::Duration,
}

trivial_move! {
    (),
    bool,
    char,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,

    &T where [T: ?Sized],
    *const T where [T: ?Sized],

    *mut T where [T: ?Sized],
    &mut T where [T: ?Sized],

    ::core::alloc::Layout,

    ::core::cell::UnsafeCell<T> where [T],
    ::core::cell::Cell<T> where [T],
    ::core::cell::RefCell<T> where [T],
    ::core::cell::Ref<'_, T> where [T],
    ::core::cell::RefMut<'_, T> where [T],

    ::core::marker::PhantomData<T> where [T: ?Sized],
    ::core::marker::PhantomPinned,

    ::core::mem::Discriminant<T> where [T],
    ::core::mem::ManuallyDrop<T> where [T],
    ::core::mem::MaybeUninit<T> where [T],

    ::core::num::NonZeroI8,
    ::core::num::NonZeroI16,
    ::core::num::NonZeroI32,
    ::core::num::NonZeroI64,
    ::core::num::NonZeroI128,
    ::core::num::NonZeroIsize,
    ::core::num::NonZeroU8,
    ::core::num::NonZeroU16,
    ::core::num::NonZeroU32,
    ::core::num::NonZeroU64,
    ::core::num::NonZeroU128,
    ::core::num::NonZeroUsize,
    ::core::num::Wrapping<T> where [T],

    ::core::option::Option<T> where [T],

    ::core::pin::Pin<T> where [T],
    ::core::ptr::NonNull<T> where [T],

    ::core::result::Result<T, E> where [T, E],

    ::core::sync::atomic::AtomicI8,
    ::core::sync::atomic::AtomicI16,
    ::core::sync::atomic::AtomicI32,
    ::core::sync::atomic::AtomicI64,
    ::core::sync::atomic::AtomicIsize,
    ::core::sync::atomic::AtomicU8,
    ::core::sync::atomic::AtomicU16,
    ::core::sync::atomic::AtomicU32,
    ::core::sync::atomic::AtomicU64,
    ::core::sync::atomic::AtomicUsize,
    ::core::sync::atomic::AtomicPtr<T> where [T],

    ::core::time::Duration,
}
