//! Explicit value-copy adapters for reference-counted owners.

use std::{marker::PhantomData, rc::Rc as NativeRc, sync::Arc as NativeArc};

use crate::{BoxInput, EncodeSource, Intrlay};

macro_rules! shared_owner {
    ($codec:ident, $owner:ident) => {
        /// Serializes a reference-counted owner by copying its contents at each occurrence.
        ///
        /// Select explicitly with `#[intrlay(with = ...)]`. Sized values use the
        /// Box representation; `[C]` and `str` use sequence and string representations.
        /// `C` is the pointee codec, so element adapters compose recursively.
        ///
        /// Shared identity is not retained. Decoding produces independent allocations,
        /// even when multiple fields originally pointed to the same value. Cycles and
        /// weak references are not supported; reachable values must remain stable during
        /// measurement and writing. This adapter does not add synchronization.
        pub struct $codec<C: ?Sized>(PhantomData<C>);

        impl<C: crate::schema::Describe> crate::schema::Describe for $codec<C> {
            fn describe(registry: &mut crate::schema::Registry) -> crate::schema::Type {
                <Box<C> as crate::schema::Describe>::describe(registry)
            }
        }
        impl<C: crate::schema::Describe> crate::schema::Describe for $codec<[C]> {
            fn describe(registry: &mut crate::schema::Registry) -> crate::schema::Type {
                <Vec<C> as crate::schema::Describe>::describe(registry)
            }
        }
        impl crate::schema::Describe for $codec<str> {
            fn describe(registry: &mut crate::schema::Registry) -> crate::schema::Type {
                <String as crate::schema::Describe>::describe(registry)
            }
        }

        // SAFETY: Box<C> defines the representation. The pointee is encoded anew
        // on every visit and decoded into a fresh owner, never a shared wire reference.
        unsafe impl<C: Intrlay> Intrlay for $codec<C> {
            type Value = $owner<C::Value>;
            type View<'a> = C::View<'a>;
            type ViewMut<'a> = C::ViewMut<'a>;
            const LAYOUT: crate::layout::Layout = <Box<C> as Intrlay>::LAYOUT;
            const INITIALIZATION: crate::Initialization = <Box<C> as Intrlay>::INITIALIZATION;
            fn measure(value: &Self::Value, cursor: &mut crate::SizeCursor) -> Result<(), crate::EncodeError> {
                <BoxInput<crate::ValueSource<'_, C>> as EncodeSource<Box<C>>>::measure(
                    &BoxInput(crate::ValueSource::new(value.as_ref())), cursor,
                )
            }
            fn write(value: &Self::Value, writer: &mut crate::Writer<'_>, offset: usize) -> Result<(), crate::EncodeError> {
                <BoxInput<crate::ValueSource<'_, C>> as EncodeSource<Box<C>>>::write(
                    &BoxInput(crate::ValueSource::new(value.as_ref())), writer, offset,
                )
            }
            unsafe fn view_unchecked(bytes: &[u8], offset: usize) -> Self::View<'_> {
                // SAFETY: The caller guarantees the same valid indirect value as Box<C>.
                unsafe { <Box<C> as Intrlay>::view_unchecked(bytes, offset) }
            }
            unsafe fn view_mut_unchecked(bytes: &mut [u8], offset: usize) -> Self::ViewMut<'_> {
                // SAFETY: Wire occurrences are independent; the borrow is exclusive.
                unsafe { <Box<C> as Intrlay>::view_mut_unchecked(bytes, offset) }
            }
            fn validate_fields(validator: &mut crate::Validator<'_>, offset: usize) -> Result<(), crate::DecodeError> {
                <Box<C> as Intrlay>::validate_fields(validator, offset)
            }
            fn decode_from(decoder: &mut crate::Decoder<'_>, offset: usize) -> Result<Self::Value, crate::DecodeError> {
                let target = decoder.reference(offset, Self::LAYOUT.size())?;
                decoder.decode::<C>(target).map($owner::new)
            }
            unsafe fn decode_unchecked(bytes: &[u8], offset: usize) -> Self::Value {
                // SAFETY: The caller supplies a valid relative reference and pointee.
                unsafe {
                    let target = offset + u32::view_unchecked(bytes, offset) as usize;
                    $owner::new(C::decode_unchecked(bytes, target))
                }
            }
        }

        crate::containers::string_codec!($codec<str>, $owner);
        crate::containers::slice_codec!(C, $codec<[C]>, $owner, allocation => {
            match $owner::get_mut(&mut allocation) {
                Some(slots) => slots,
                // The owner was just allocated and has not been cloned or downgraded.
                None => unreachable!("new slice allocation has no other owners"),
            }
        });
        crate::containers::string_sources!($codec<str>);
        crate::containers::sequence_sources!(C, $codec<[C]>);
        crate::containers::string_source!($owner<str> => String);
        crate::containers::slice_source!($owner);

        // SAFETY: Borrowing the pointee and using BoxInput never copies the native
        // reference count or pointer. The wire encoder creates its own occurrence.
        unsafe impl<C: Intrlay, S: EncodeSource<C>> EncodeSource<$codec<C>> for $owner<S> {
            const INITIALIZATION: crate::Initialization = <BoxInput<S> as EncodeSource<Box<C>>>::INITIALIZATION;
            fn measure(&self, cursor: &mut crate::SizeCursor) -> Result<(), crate::EncodeError> {
                <BoxInput<&S> as EncodeSource<Box<C>>>::measure(&BoxInput(self.as_ref()), cursor)
            }
            fn write(&self, writer: &mut crate::Writer<'_>, offset: usize) -> Result<(), crate::EncodeError> {
                <BoxInput<&S> as EncodeSource<Box<C>>>::write(&BoxInput(self.as_ref()), writer, offset)
            }
        }
        // SAFETY: An input may borrow or construct the pointee without allocating
        // a native reference-counted owner; the Box representation is identical.
        unsafe impl<C: Intrlay, S: EncodeSource<C>> EncodeSource<$codec<C>> for BoxInput<S> {
            const INITIALIZATION: crate::Initialization = <Self as EncodeSource<Box<C>>>::INITIALIZATION;
            fn measure(&self, cursor: &mut crate::SizeCursor) -> Result<(), crate::EncodeError> {
                <Self as EncodeSource<Box<C>>>::measure(self, cursor)
            }
            fn write(&self, writer: &mut crate::Writer<'_>, offset: usize) -> Result<(), crate::EncodeError> {
                <Self as EncodeSource<Box<C>>>::write(self, writer, offset)
            }
        }
    };
}

shared_owner!(Rc, NativeRc);
shared_owner!(Arc, NativeArc);
