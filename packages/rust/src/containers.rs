//! Ownership-independent string and slice codecs. Owners never affect the schema.

macro_rules! string_codec {
    ($codec:ty, $owner:ident) => {
        // SAFETY: The String codec defines the representation and validation.
        // Native ownership affects only materialization, never the encoded bytes.
        unsafe impl crate::Intrlay for $codec {
            type Value = $owner<str>;
            type View<'a> = &'a str;
            type ViewMut<'a> = &'a str;
            const LAYOUT: crate::layout::Layout = <String as crate::Intrlay>::LAYOUT;
            const INITIALIZATION: crate::Initialization =
                <String as crate::Intrlay>::INITIALIZATION;
            fn measure(
                value: &Self::Value,
                cursor: &mut crate::SizeCursor,
            ) -> Result<(), crate::EncodeError> {
                <str as crate::EncodeSource<String>>::measure(value, cursor)
            }
            fn write(
                value: &Self::Value,
                writer: &mut crate::Writer<'_>,
                offset: usize,
            ) -> Result<(), crate::EncodeError> {
                <str as crate::EncodeSource<String>>::write(value, writer, offset)
            }
            unsafe fn view_unchecked(bytes: &[u8], offset: usize) -> &str {
                // SAFETY: The caller supplies the standard validated string representation.
                unsafe { <String as crate::Intrlay>::view_unchecked(bytes, offset) }
            }
            unsafe fn view_mut_unchecked(bytes: &mut [u8], offset: usize) -> &str {
                // SAFETY: Strings expose immutable contents within the exclusive borrow.
                unsafe { <String as crate::Intrlay>::view_unchecked(bytes, offset) }
            }
            fn validate_fields(
                validator: &mut crate::Validator<'_>,
                offset: usize,
            ) -> Result<(), crate::DecodeError> {
                <String as crate::Intrlay>::validate_fields(validator, offset)
            }
            fn decode_from(
                decoder: &mut crate::Decoder<'_>,
                offset: usize,
            ) -> Result<Self::Value, crate::DecodeError> {
                Ok($owner::from(decoder.string(offset)?))
            }
            unsafe fn decode_unchecked(bytes: &[u8], offset: usize) -> Self::Value {
                // SAFETY: The caller guarantees the complete UTF-8 string encoding.
                $owner::from(unsafe { <String as crate::Intrlay>::view_unchecked(bytes, offset) })
            }
        }
    };
}

macro_rules! slice_codec {
    ($element:ident, $codec:ty, $owner:ident, $allocation:ident => $slots:expr) => {
        // SAFETY: Vec<$element> defines the header, element layout, and validation.
        // Every decode creates an independent allocation and preserves prefix cleanup.
        unsafe impl<$element: crate::Intrlay> crate::Intrlay for $codec {
            type Value = $owner<[$element::Value]>;
            type View<'a> = crate::SequenceView<'a, $element>;
            type ViewMut<'a> = crate::SequenceMut<'a, $element>;
            const LAYOUT: crate::layout::Layout = <Vec<$element> as crate::Intrlay>::LAYOUT;
            const INITIALIZATION: crate::Initialization =
                <Vec<$element> as crate::Intrlay>::INITIALIZATION;
            fn measure(
                value: &Self::Value,
                cursor: &mut crate::SizeCursor,
            ) -> Result<(), crate::EncodeError> {
                crate::builtins::measure_sequence::<$element>(value, cursor)
            }
            fn write(
                value: &Self::Value,
                writer: &mut crate::Writer<'_>,
                offset: usize,
            ) -> Result<(), crate::EncodeError> {
                crate::builtins::write_sequence::<$element>(value, writer, offset)
            }
            unsafe fn view_unchecked(bytes: &[u8], offset: usize) -> Self::View<'_> {
                // SAFETY: The caller guarantees the same valid sequence representation.
                unsafe { <Vec<$element> as crate::Intrlay>::view_unchecked(bytes, offset) }
            }
            unsafe fn view_mut_unchecked(bytes: &mut [u8], offset: usize) -> Self::ViewMut<'_> {
                // SAFETY: The caller exclusively borrows the complete valid sequence.
                unsafe { <Vec<$element> as crate::Intrlay>::view_mut_unchecked(bytes, offset) }
            }
            fn validate_fields(
                validator: &mut crate::Validator<'_>,
                offset: usize,
            ) -> Result<(), crate::DecodeError> {
                <Vec<$element> as crate::Intrlay>::validate_fields(validator, offset)
            }
            fn decode_from(
                decoder: &mut crate::Decoder<'_>,
                offset: usize,
            ) -> Result<Self::Value, crate::DecodeError> {
                let target = decoder.reference(offset, Self::LAYOUT.size())?;
                let length = u32::from_le_bytes(decoder.read(offset + 4)?) as usize;
                decoder.array_region(target, length, $element::LAYOUT.size())?;
                let mut $allocation = $owner::<[$element::Value]>::new_uninit_slice(length);
                crate::Decoder::decode_elements::<$element>(decoder, target, $slots)?;
                // SAFETY: The decoder initialized every slot; errors and unwinding
                // drop the initialized prefix before releasing uninitialized storage.
                Ok(unsafe { $allocation.assume_init() })
            }
            unsafe fn decode_unchecked(bytes: &[u8], offset: usize) -> Self::Value {
                // SAFETY: The caller supplies a complete valid sequence header.
                let (target, length) = unsafe {
                    (
                        offset + <u32 as crate::Intrlay>::view_unchecked(bytes, offset) as usize,
                        <u32 as crate::Intrlay>::view_unchecked(bytes, offset + 4) as usize,
                    )
                };
                let mut $allocation = $owner::<[$element::Value]>::new_uninit_slice(length);
                // SAFETY: The caller guarantees valid elements, and the helper
                // initializes all slots while retaining cleanup if a codec unwinds.
                unsafe {
                    crate::decode::decode_elements_unchecked::<$element>(bytes, target, $slots);
                    $allocation.assume_init()
                }
            }
        }
    };
}

macro_rules! string_source {
    ($source:ty => $codec:ty) => {
        // SAFETY: String's source codec writes the same valid UTF-8 representation.
        unsafe impl crate::EncodeSource<$codec> for $source {
            const INITIALIZATION: crate::Initialization =
                <String as crate::Intrlay>::INITIALIZATION;
            fn measure(&self, cursor: &mut crate::SizeCursor) -> Result<(), crate::EncodeError> {
                <str as crate::EncodeSource<String>>::measure(self.as_ref(), cursor)
            }
            fn write(
                &self,
                writer: &mut crate::Writer<'_>,
                offset: usize,
            ) -> Result<(), crate::EncodeError> {
                <str as crate::EncodeSource<String>>::write(self.as_ref(), writer, offset)
            }
        }
    };
}

macro_rules! string_sources {
    ($codec:ty) => {
        crate::containers::string_source!(str => $codec);
        crate::containers::string_source!(String => $codec);
        crate::containers::string_source!(Box<str> => $codec);
        crate::containers::string_source!(std::rc::Rc<str> => $codec);
        crate::containers::string_source!(std::sync::Arc<str> => $codec);
    };
}

macro_rules! sequence_source {
    ($element:ident, [$($parameters:tt)*] $source:ty => $codec:ty) => {
        // SAFETY: The selected owner uses exactly the ordinary Vec<$element> wire layout.
        unsafe impl<$element: crate::Intrlay, $($parameters)*> crate::EncodeSource<$codec> for $source
        where Self: crate::EncodeSource<Vec<$element>>
        {
            const INITIALIZATION: crate::Initialization = <Self as crate::EncodeSource<Vec<$element>>>::INITIALIZATION;
            fn measure(&self, cursor: &mut crate::SizeCursor) -> Result<(), crate::EncodeError> {
                <Self as crate::EncodeSource<Vec<$element>>>::measure(self, cursor)
            }
            fn write(&self, writer: &mut crate::Writer<'_>, offset: usize) -> Result<(), crate::EncodeError> {
                <Self as crate::EncodeSource<Vec<$element>>>::write(self, writer, offset)
            }
        }
    };
}

macro_rules! sequence_sources {
    ($element:ident, $codec:ty) => {
        crate::containers::sequence_source!($element, [S] [S] => $codec);
        crate::containers::sequence_source!($element, [S] Vec<S> => $codec);
        crate::containers::sequence_source!($element, [S] Box<[S]> => $codec);
        crate::containers::sequence_source!($element, [S] std::rc::Rc<[S]> => $codec);
        crate::containers::sequence_source!($element, [S] std::sync::Arc<[S]> => $codec);
        crate::containers::sequence_source!($element, [I] crate::SequenceInput<I> => $codec);
        crate::containers::sequence_source!($element, [] crate::ZeroedSequence => $codec);
    };
}

macro_rules! slice_source {
    ($owner:ident) => {
        // SAFETY: Borrowing the pointee slice retains element-source native-copy proofs.
        unsafe impl<C: crate::Intrlay, S: crate::EncodeSource<C>> crate::EncodeSource<Vec<C>>
            for $owner<[S]>
        {
            const INITIALIZATION: crate::Initialization =
                <[S] as crate::EncodeSource<Vec<C>>>::INITIALIZATION;
            fn measure(&self, cursor: &mut crate::SizeCursor) -> Result<(), crate::EncodeError> {
                <[S] as crate::EncodeSource<Vec<C>>>::measure(self, cursor)
            }
            fn write(
                &self,
                writer: &mut crate::Writer<'_>,
                offset: usize,
            ) -> Result<(), crate::EncodeError> {
                <[S] as crate::EncodeSource<Vec<C>>>::write(self, writer, offset)
            }
        }
    };
}

pub(crate) use {
    sequence_source, sequence_sources, slice_codec, slice_source, string_codec, string_source,
    string_sources,
};

string_codec!(Box<str>, Box);
slice_codec!(C, Box<[C]>, Box, allocation => &mut allocation);
string_sources!(Box<str>);
sequence_sources!(C, Box<[C]>);
string_source!(Box<str> => String);
slice_source!(Box);
