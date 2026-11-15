use crate::{
    DecodeError, Decoder, EncodeError, EncodeSource, Initialization, Intrlay, SizeCursor,
    ValueView, Writer,
    layout::{Layout, field_offset, record},
};

mod sealed {
    pub trait Tuple {}
}

/// A statically selected field of a built-in tuple codec.
///
/// Implemented for tuples of one through twelve elements. The index is checked
/// by trait resolution, so an out-of-range field cannot be accessed. Implementations
/// are sealed because the field type and offset are part of the view's safety proof.
pub trait TupleField<const INDEX: usize>: Intrlay + sealed::Tuple {
    /// Codec for the selected field, independent of its native storage type.
    type Field: Intrlay;
    /// Byte offset of the field within the tuple's inline header.
    const OFFSET: usize;
}

impl<C: Intrlay, B: AsRef<[u8]>> ValueView<C, B> {
    /// Borrows a tuple field selected at compile time, for example `field::<0>()`.
    #[must_use]
    #[inline]
    pub fn field<const INDEX: usize>(&self) -> <C::Field as Intrlay>::View<'_>
    where
        C: TupleField<INDEX>,
    {
        // SAFETY: The view is valid and sealed TupleField implementations identify
        // the exact codec and offset of an inline field of C.
        unsafe { C::Field::view_unchecked(self.as_bytes(), self.byte_offset() + C::OFFSET) }
    }
}

impl<C: Intrlay, B: AsRef<[u8]> + AsMut<[u8]>> ValueView<C, B> {
    /// Exclusively borrows a tuple field without creating overlapping mutable views.
    #[inline]
    pub fn field_mut<const INDEX: usize>(&mut self) -> <C::Field as Intrlay>::ViewMut<'_>
    where
        C: TupleField<INDEX>,
    {
        let offset = self.byte_offset() + C::OFFSET;
        // SAFETY: The sealed field mapping is valid; the returned child can only
        // perform layout-preserving writes and exclusively borrows the parent.
        unsafe { C::Field::view_mut_unchecked(self.as_bytes_mut_unchecked(), offset) }
    }
}

macro_rules! tuple_fields {
    ([$($all:ident),+]; $index:tt => $field:ident $(, $rest_index:tt => $rest:ident)*) => {
        impl<$($all: Intrlay),+> TupleField<$index> for ($($all,)+) {
            type Field = $field;
            const OFFSET: usize = field_offset(&[$($all::LAYOUT),+], $index);
        }
        tuple_fields!([$($all),+]; $($rest_index => $rest),*);
    };
    ([$($all:ident),+];) => {};
}

macro_rules! tuple {
    ($(($index:tt, $codec:ident, $source:ident)),+) => {
        impl<$($codec: Intrlay),+> sealed::Tuple for ($($codec,)+) {}
        tuple_fields!([$($codec),+]; $($index => $codec),+);

        // SAFETY: Every operation uses declaration-ordered wire offsets and the
        // constituent codecs. No assumption is made about Rust's tuple layout.
        unsafe impl<$($codec: Intrlay),+> Intrlay for ($($codec,)+) {
            type Value = ($($codec::Value,)+);
            type View<'a> = ValueView<Self, &'a [u8]>;
            type ViewMut<'a> = ValueView<Self, &'a mut [u8]>;
            const LAYOUT: Layout = record(&[$($codec::LAYOUT),+], 1);
            const ALL_BITS_VALID: bool = true $(&& $codec::ALL_BITS_VALID)+;
            const ZEROABLE: bool = true $(&& $codec::ZEROABLE)+;
            const HAS_PAYLOAD: bool = false $(|| $codec::HAS_PAYLOAD)+;
            const IN_PLACE_WRITE: bool = true $(&& $codec::IN_PLACE_WRITE)+;
            const FIXED_DECODE_DEPTH: Option<usize> = if Self::ALL_BITS_VALID {
                crate::decode::fixed_decode_depth(&[$($codec::FIXED_DECODE_DEPTH),+])
            } else {
                None
            };
            const WRITES_FIXED_LAYOUT: bool = true $(&& $codec::WRITES_FIXED_LAYOUT)+
                && Self::LAYOUT.size() == 0 $(+ $codec::LAYOUT.size())+;
            const INITIALIZATION: Initialization = Initialization::record(
                Self::LAYOUT, &[$(($codec::LAYOUT, $codec::INITIALIZATION)),+],
            );

            fn measure(value: &Self::Value, cursor: &mut SizeCursor) -> Result<(), EncodeError> {
                $($codec::measure(&value.$index, cursor)?;)+
                Ok(())
            }
            fn write(value: &Self::Value, writer: &mut Writer<'_>, offset: usize) -> Result<(), EncodeError> {
                $($codec::write(&value.$index, writer, offset + <Self as TupleField<$index>>::OFFSET)?;)+
                Ok(())
            }
            unsafe fn view_unchecked(bytes: &[u8], offset: usize) -> Self::View<'_> {
                // SAFETY: The caller guarantees the tuple and all descendants are valid.
                unsafe { ValueView::new_unchecked(bytes, offset) }
            }
            unsafe fn view_mut_unchecked(bytes: &mut [u8], offset: usize) -> Self::ViewMut<'_> {
                // SAFETY: The caller grants exclusive access to the complete valid tuple.
                unsafe { ValueView::new_unchecked(bytes, offset) }
            }
            fn validate_fields(validator: &mut crate::Validator<'_>, offset: usize) -> Result<(), DecodeError> {
                $(validator.field::<$codec>(offset + <Self as TupleField<$index>>::OFFSET)?;)+
                Ok(())
            }
            fn decode_from(decoder: &mut Decoder<'_>, offset: usize) -> Result<Self::Value, DecodeError> {
                // Rust drops earlier fields if decoding a later field fails or unwinds.
                Ok(($(decoder.decode::<$codec>(offset + <Self as TupleField<$index>>::OFFSET)?,)+))
            }
            unsafe fn decode_unchecked(bytes: &[u8], offset: usize) -> Self::Value {
                // SAFETY: The caller guarantees every field and its descendants are valid.
                unsafe { ($($codec::decode_unchecked(bytes, offset + <Self as TupleField<$index>>::OFFSET),)+) }
            }
        }

        // SAFETY: Each field source writes the corresponding codec's representation.
        unsafe impl<$($codec: Intrlay, $source: EncodeSource<$codec>),+>
            EncodeSource<($($codec,)+)> for ($($source,)+)
        {
            const WRITES_FIXED_LAYOUT: bool = true $(&& $source::WRITES_FIXED_LAYOUT)+
                && <($($codec,)+) as Intrlay>::LAYOUT.size() == 0 $(+ $codec::LAYOUT.size())+;
            const INITIALIZATION: Initialization = Initialization::record(
                <($($codec,)+) as Intrlay>::LAYOUT,
                &[$(($codec::LAYOUT, $source::INITIALIZATION)),+],
            );
            fn measure(&self, cursor: &mut SizeCursor) -> Result<(), EncodeError> {
                $(self.$index.measure(cursor)?;)+
                Ok(())
            }
            fn write(&self, writer: &mut Writer<'_>, offset: usize) -> Result<(), EncodeError> {
                let layouts = &[$($codec::LAYOUT),+];
                $(self.$index.write(writer, offset + field_offset(layouts, $index))?;)+
                Ok(())
            }
        }
    };
}

macro_rules! tuples {
    ([$($done:tt),*]; $next:tt $(, $rest:tt)*) => {
        tuple!($($done,)* $next);
        tuples!([$($done,)* $next]; $($rest),*);
    };
    ([$($done:tt),*];) => {};
}

tuples!([]; (0, A, SourceA), (1, B, SourceB), (2, C, SourceC), (3, D, SourceD),
    (4, E, SourceE), (5, F, SourceF), (6, G, SourceG), (7, H, SourceH),
    (8, I, SourceI), (9, J, SourceJ), (10, K, SourceK), (11, L, SourceL));
