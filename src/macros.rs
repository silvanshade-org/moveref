/// Macro for binding a variable to a fresh [`MoveRef`](crate::MoveRef).
///
/// - `bind!(x = &move *ptr)` creates an `x: MoveRef<T>` given `ptr: impl
///   (DerefMove + DerefMut<Target = T>)`
///
/// The first form transfers a uniquely owned referent into the bound
/// move-reference.
///
/// - `bind!(x = &move val)` creates an `x: MoveRef<T>` given `val: T`
///
/// The above invocation moves any value into a fresh
/// [`MoveRef`](crate::MoveRef) bound to `x`.
///
/// - `bind!(x = con)` creates an `x: Pin<MoveRef<T>>` given `con: impl
///   New<Output = T>`
///
/// The constructor form initializes the value in the bound move-reference’s
/// slot.
///
/// - `bind!(mut x: T = ...)` (with right-hand side of `&move *ptr` or `&move
///   val` or `con`)
///
/// The above generalization can be used with any earlier invocation form to add
/// mutability and typing annotations.
///
/// # Specification
/// - requires: the right-hand side supplies an owning pointer, a value, or a
///   valid constructor.
/// - ensures: the binding owns one referent through its backing slot until
///   scope exit.
/// - provides: a `MoveRef` for moved values or pointers, or a pinned one for
///   constructors.
///
/// # Adequacy
/// - hypothesis: each expansion must end ownership once at the binding’s scope
///   exit.
/// - witness: `macros::test::macros::binding_forms_drop_the_owned_referent_on_scope_exit`.
#[macro_export]
macro_rules! bind {
    (mut $name:ident $(: $ty:ty)? = &move *$expr:expr) => {
        $crate::bind!(@move(mut) $name, $($ty)?, $expr)
    };
    ($name:ident $(: $ty:ty)? = &move *$expr:expr) => {
        $crate::bind!(@move $name, $($ty)?, $expr)
    };
    (mut $name:ident $(: $ty:ty)? = &move $expr:expr) => {
        $crate::bind!(@put(mut) $name, $($ty)?, $expr)
    };
    ($name:ident $(: $ty:ty)? = &move $expr:expr) => {
        $crate::bind!(@put $name, $($ty)?, $expr)
    };
    (mut $name:ident $(: $ty:ty)? = $expr:expr) => {
        $crate::bind!(@emplace(mut) $name, $($ty)?, $expr);
    };
    ($name:ident $(: $ty:ty)? = $expr:expr) => {
        $crate::bind!(@emplace $name, $($ty)?, $expr);
    };
    (@move $(($mut:tt))? $name:ident, $($ty:ty)?, $expr:expr) => {
        $crate::bind_slot!(#[dropping] slot);
        let $($mut)? $name $(: $ty)? = $crate::DerefMove::deref_move($expr, slot);
    };
    (@put $(($mut:tt))? $name:ident, $($ty:ty)?, $expr:expr) => {
        $crate::bind_slot!(slot);
        let $($mut)? $name $(: $ty)? = slot.put($expr);
    };
    (@emplace $(($mut:tt))? $name:ident, $($ty:ty)?, $expr:expr) => {
        $crate::bind_slot!(slot);
        let $($mut)? $name $(: $ty)? = slot.emplace($expr);
    };
}

/// Macro for creating a fresh [`MoveRef`](crate::MoveRef) expression.
///
/// Because a `v: MoveRef<'frame, T>` always has an associated lifetime
/// `'frame`, this macro can generally only be used where it would be
/// immediately consumed by some enclosing expression, such as in the position
/// of a function argument.
///
/// Trying to use this as `let v = expr!(...)` will not work because the
/// lifetime `'frame` will not expand to the enclosing let binding. Hence the
/// need for the separate `bind!(v = ...)` macro.
///
/// Otherwise, the usage is generally the same as `bind!(...)`:
///
/// - `expr!(&move *ptr)` creates a `MoveRef<T>` given `ptr: impl (DerefMove +
///   DerefMut<Target = T>)`
///
/// The first form transfers a uniquely owned referent into the expression.
///
/// - `expr!(&move val)` creates an `MoveRef<T>` given `val: T`
///
/// The above invocation moves any value into a fresh
/// [`MoveRef`](crate::MoveRef).
///
/// - `expr!(con)` creates an `x: Pin<MoveRef<T>>` given `con: impl New<Output =
///   T>`
///
/// The constructor form initializes the value in the temporary slot.
///
/// # Specification
/// - requires: consume the returned reference in the enclosing expression.
/// - ensures: preserves unique ownership and drops the referent once on scope
///   exit.
/// - provides: a move-reference for moved values or pointers, or a pinned one
///   for constructors.
///
/// # Adequacy
/// - hypothesis: any expansion dropping too soon or too late changes the
///   destructor count.
/// - witness: `macros::test::macros::expression_forms_end_the_ownership_lifecycle_once`.
#[macro_export]
macro_rules! expr {
    (&move *$expr:expr) => {
        $crate::DerefMove::deref_move(
            $expr,
            $crate::expr_slot!(
                #[dropping]
            )
        )
    };
    (&move $expr:expr) => {
        $crate::expr_slot!().put($expr)
    };
    ($expr:expr) => {
        $crate::expr_slot!().emplace($expr)
    };
}

/// Macro for binding a variable to a fresh [`Slot`](crate::Slot) for storage of
/// a [`MoveRef`](crate::MoveRef).
///
/// - `bind_slot!(x)`
///
/// The above invocation binds a slot to the variable `x`.
///
/// - `bind_slot!(#[dropping] x)` binds a slot whose stored `T` is destroyed
///   when storage ends. Use it only for a resource distinct from the `MoveRef`
///   referent, such as `Box<MaybeUninit<U>>`; a directly stored referent would
///   be destroyed twice.
///
/// Both invocation forms allow type annotations.
///
/// # Specification
/// - requires: `#[dropping]` stores a `T` that remains valid after the referent
///   destructor.
/// - ensures: creates one fresh slot for each name and keeps its backing
///   storage in scope.
/// - provides: a `Keep` slot by default, or a `Drop` slot for independent
///   resources.
///
/// # Adequacy
/// - hypothesis: a slot ending before its owner changes destruction count or
///   address validity.
/// - witness: `macros::test::macros::binding_forms_drop_the_owned_referent_on_scope_exit`.
#[macro_export]
macro_rules! bind_slot {
    (#[dropping] $($name:ident : $ty:ty),* $(,)?) => {
        $(
            let kind = $crate::SlotStorageKind::Drop;
            let mut storage = $crate::SlotStorage::<$ty>::new(kind);
            let $name = storage.slot();
        )*
    };
    (#[dropping] $($name:ident),* $(,)?) => {
        $(
            let kind = $crate::SlotStorageKind::Drop;
            let mut storage = $crate::SlotStorage::new(kind);
            let $name = storage.slot();
        )*
    };
    ($($name:ident : $ty:ty),* $(,)?) => {
        $(
            let kind = $crate::SlotStorageKind::Keep;
            let mut storage = $crate::SlotStorage::<$ty>::new(kind);
            let $name = storage.slot();
        )*
    };
    ($($name:ident),* $(,)?) => {
        $(
            let kind = $crate::SlotStorageKind::Keep;
            let mut storage = $crate::SlotStorage::new(kind);
            let $name = storage.slot();
        )*
    };
}

/// Create a temporary storage slot expression for immediate placement.
///
/// # Specification
/// - requires: a `Drop` slot only stores resources still valid after its
///   move-reference ends.
/// - ensures: the resulting temporary slot lives through its enclosing
///   expression.
/// - provides: fresh `Keep` storage, or independent-resource `Drop` storage.
///
/// # Adequacy
/// - hypothesis: a temporary slot dropped too early would invalidate the owning
///   reference.
/// - witness: `macros::test::macros::expression_forms_end_the_ownership_lifecycle_once`.
#[macro_export]
macro_rules! expr_slot {
    (#[dropping]) => {
        $crate::SlotStorage::new($crate::SlotStorageKind::Drop).slot()
    };
    () => {
        $crate::SlotStorage::new($crate::SlotStorageKind::Keep).slot()
    };
}

/// Boilerplate macro for defining trivial [`CopyNew`](crate::CopyNew)
/// instances.
///
/// # Specification
/// - requires: each listed type implements `Clone`.
/// - ensures: copies into fresh pinned storage without consuming the source.
/// - provides: `CopyNew` implementations for listed types.
macro_rules! trivial_copy {
    ($($ty:ty $(where [$($targs:tt)*])?),* $(,)?) => {
        $(
            impl<$($($targs)*)?> $crate::new::CopyNew for $ty where Self: ::core::clone::Clone {
                #[inline]
                unsafe fn copy_new(
                    this: &Self,
                    that: ::core::pin::Pin<&mut ::core::mem::MaybeUninit<Self>>,
                ) {
                    // SAFETY: the caller provides fresh pinned storage, and this operation only
                    // obtains its uninitialized address before writing the cloned value.
                    let that = unsafe { ::core::pin::Pin::into_inner_unchecked(that) };
                    let data = this.clone();
                    that.write(data);
                }
            }
        )*
    }
}

/// Generate direct owning moves for selected [`MoveNew`](crate::MoveNew) types.
///
/// # Specification
/// - requires: the selected type can be moved by value out of its owning
///   reference.
/// - ensures: consumes one source and writes one valid value into fresh pinned
///   storage.
/// - provides: `MoveNew` implementations for listed types.
macro_rules! trivial_move {
    ($($ty:ty $(where [$($targs:tt)*])?),* $(,)?) => {
        $(
            impl<$($($targs)*)?> $crate::new::MoveNew for $ty {
                #[inline]
                unsafe fn move_new(
                    this: ::core::pin::Pin<$crate::move_ref::MoveRef<'_, Self>>,
                    that: ::core::pin::Pin<&mut ::core::mem::MaybeUninit<Self>>,
                ) {
                    // SAFETY: moving from this owning reference consumes its referent exactly once.
                    let this = unsafe { ::core::pin::Pin::into_inner_unchecked(this) };
                    // SAFETY: the caller provides fresh storage; no initialized value is moved.
                    let that = unsafe { ::core::pin::Pin::into_inner_unchecked(that) };
                    let data = $crate::move_ref::MoveRef::into_inner(this);
                    that.write(data);
                }
            }
        )*
    }
}

#[cfg(test)]
mod test
{
    use crate::*;

    mod macros
    {
        use super::*;

        const VAL: bool = true;

        #[test]
        fn deref_move_expr()
        {
            assert_eq!(VAL, *expr!(&move *Box::new(VAL)));
        }

        #[test]
        fn trivial_copy()
        {
            let this = &true;
            let that = ::core::mem::MaybeUninit::uninit();
            let mut that = ::core::pin::pin!(that);
            // SAFETY: the bool CopyNew implementation writes into this fresh pinned
            // storage.
            unsafe { new::CopyNew::copy_new(this, that.as_mut()) };
            // SAFETY: copy_new returned after initializing the destination.
            let that = unsafe { that.assume_init() };
            assert_eq!(this, &that);
        }

        #[test]
        fn trivial_move()
        {
            bind!(this = new::of(VAL));
            let that = ::core::mem::MaybeUninit::uninit();
            let mut that = ::core::pin::pin!(that);
            // SAFETY: the bool MoveNew implementation consumes the source and initializes
            // that.
            unsafe { new::MoveNew::move_new(this, that.as_mut()) };
            // SAFETY: move_new returned after initializing the destination.
            let that = unsafe { that.assume_init() };
            assert_eq!(VAL, that);
        }

        #[repr(transparent)]
        struct Counted<'counter>(&'counter core::cell::Cell<usize>);

        impl Drop for Counted<'_>
        {
            fn drop(&mut self)
            {
                self.0.set(self.0.get().saturating_add(1));
            }
        }

        #[test]
        fn expression_forms_end_the_ownership_lifecycle_once()
        {
            let put_drops = core::cell::Cell::new(0);
            drop(expr!(&move Counted(&put_drops)));
            assert_eq!(put_drops.get(), 1);

            let box_drops = core::cell::Cell::new(0);
            drop(expr!(&move *Box::new(Counted(&box_drops))));
            assert_eq!(box_drops.get(), 1);

            let new_drops = core::cell::Cell::new(0);
            drop(expr!(new::of(Counted(&new_drops))));
            assert_eq!(new_drops.get(), 1);
        }

        #[test]
        fn binding_forms_drop_the_owned_referent_on_scope_exit()
        {
            let put_drops = core::cell::Cell::new(0);
            let before = {
                bind!(placed = &move Counted(&put_drops));
                placed.0.get()
            };
            assert_eq!(before, 0);
            assert_eq!(put_drops.get(), 1);

            let box_drops = core::cell::Cell::new(0);
            let before = {
                bind!(moved = &move *Box::new(Counted(&box_drops)));
                moved.0.get()
            };
            assert_eq!(before, 0);
            assert_eq!(box_drops.get(), 1);

            let new_drops = core::cell::Cell::new(0);
            let before = {
                bind!(pinned = new::of(Counted(&new_drops)));
                pinned.0.get()
            };
            assert_eq!(before, 0);
            assert_eq!(new_drops.get(), 1);
        }
    }
}
