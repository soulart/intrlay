//! Ownership wrappers and positional fields retain the ordinary tree wire layout.

use std::error::Error;

use intrlay::{BoxInput, Intrlay, SequenceInput, TupleField, ValueView, ZeroedSequence, adapters};

#[test]
fn boxed_slices_and_strings_share_borrowed_sequence_and_string_inputs() -> Result<(), Box<dyn Error>>
{
    for text in ["", "a\0b", "Unicode 한글 🦀"] {
        let value: Box<str> = text.into();
        let bytes = intrlay::create::<Box<str>>(&value)?;
        assert_eq!(
            bytes.as_ref(),
            intrlay::create_from::<String>(text)?.as_ref()
        );
        assert_eq!(
            bytes.as_ref(),
            intrlay::create_from::<Box<str>>(text)?.as_ref()
        );
        assert_eq!(<Box<str>>::decode(bytes.as_ref(), 0)?, value);
        assert_eq!(<Box<str>>::view(bytes.as_ref(), 0)?, text);
        for length in 0..bytes.as_ref().len() {
            assert!(<Box<str>>::decode(&bytes.as_ref()[..length], 0).is_err());
            assert!(<Box<str>>::view(&bytes.as_ref()[..length], 0).is_err());
        }
    }
    for items in [vec![], vec!["one".to_owned(), "two 🦀".to_owned()]] {
        let boxed = items.clone().into_boxed_slice();
        let bytes = intrlay::create::<Box<[String]>>(&boxed)?;
        assert_eq!(
            bytes.as_ref(),
            intrlay::create::<Vec<String>>(&items)?.as_ref()
        );
        let input = SequenceInput::new(items.iter().map(String::as_str));
        assert_eq!(
            bytes.as_ref(),
            intrlay::create_from::<Box<[String]>>(&input)?.as_ref()
        );
        assert_eq!(<Box<[String]>>::decode(bytes.as_ref(), 0)?, boxed);
        for length in 0..bytes.as_ref().len() {
            assert!(<Box<[String]>>::decode(&bytes.as_ref()[..length], 0).is_err());
            assert!(<Box<[String]>>::view(&bytes.as_ref()[..length], 0).is_err());
        }
    }
    let mut zeroed = ValueView::<Box<[u32]>, _>::create_from(&ZeroedSequence::new(3))?;
    zeroed
        .value_mut()
        .get_mut(1)
        .ok_or("missing element")?
        .set(9);
    assert_eq!(
        <Box<[u32]>>::decode(zeroed.as_bytes(), 0)?.as_ref(),
        [0, 9, 0]
    );
    Ok(())
}

#[test]
fn tuple_fields_have_wire_order_independent_of_native_layout() -> Result<(), Box<dyn Error>> {
    type Fields = (u8, u64, Box<str>, Box<[(u32, String)]>);
    assert_eq!(<Fields as TupleField<1>>::OFFSET, 8);
    assert_eq!(<Fields as TupleField<2>>::OFFSET, 16);
    const {
        assert!(!Fields::NATIVE_LAYOUT);
    }
    let value: <Fields as Intrlay>::Value =
        (7, 91, "name".into(), vec![(42, "item".into())].into());
    let mut view = ValueView::<Fields, _>::create(&value)?;
    let items = [(42_u32, "item")];
    assert_eq!(
        view.as_bytes(),
        intrlay::create_from::<Fields>(&(7_u8, 91_u64, "name", items.as_slice()))?.as_ref()
    );
    assert_eq!(view.field::<0>(), 7);
    assert_eq!(view.field::<1>(), 91);
    assert_eq!(view.field::<2>(), "name");
    assert_eq!(
        view.field::<3>().get(0).ok_or("missing item")?.field::<1>(),
        "item"
    );
    view.field_mut::<1>().set(99);
    view.field_mut::<3>()
        .get_mut(0)
        .ok_or("missing item")?
        .field_mut::<0>()
        .set(53);
    let decoded = Fields::decode(view.as_bytes(), 0)?;
    assert_eq!(decoded.1, 99);
    assert_eq!(decoded.3[0].0, 53);
    for length in 0..view.as_bytes().len() {
        assert!(Fields::view(&view.as_bytes()[..length], 0).is_err());
        assert!(Fields::decode(&view.as_bytes()[..length], 0).is_err());
    }
    let single = intrlay::create_from::<(u32,)>(&(11_u32,))?;
    assert_eq!(<(u32,)>::decode(single.as_ref(), 0)?, (11,));
    let nested = intrlay::create_from::<((u32, u32), (u32,))>(&((1_u32, 2_u32), (3_u32,)))?;
    assert_eq!(
        <((u32, u32), (u32,))>::decode(nested.as_ref(), 0)?,
        ((1, 2), (3,))
    );
    Ok(())
}

macro_rules! shared_owner {
    ($test:ident, $native:path, $codec:ident) => {
        #[test]
        fn $test() -> Result<(), Box<dyn Error>> {
            use $native as Owner;
            type Codec<C> = adapters::$codec<C>;
            #[derive(Debug, PartialEq, Intrlay)]
            struct Pair {
                #[intrlay(with = adapters::$codec<u32>)]
                first: Owner<u32>,
                #[intrlay(with = adapters::$codec<u32>)]
                second: Owner<u32>,
            }
            let shared = Owner::new(42);
            let value = Pair {
                first: shared.clone(),
                second: shared.clone(),
            };
            let bytes = intrlay::create::<Pair>(&value)?;
            assert_eq!(Owner::strong_count(&shared), 3);
            assert_eq!(
                bytes.as_ref(),
                intrlay::create_from::<(Box<u32>, Box<u32>)>(&(
                    BoxInput(42_u32),
                    BoxInput(42_u32)
                ))?
                .as_ref()
            );
            let decoded = Pair::decode(bytes.as_ref(), 0)?;
            assert_eq!(decoded, value);
            assert!(!Owner::ptr_eq(&decoded.first, &decoded.second));
            assert_eq!(Owner::strong_count(&decoded.first), 1);
            let mut view = PairView::create(&value)?;
            view.first_mut().set(71);
            assert_eq!(view.second(), 42);
            let mut overlap = bytes.as_ref().to_vec();
            overlap[4..8].copy_from_slice(&4_u32.to_le_bytes());
            assert!(PairView::from_bytes(overlap).is_err());

            let values: Owner<[u32]> = Owner::from([1, 2, 3].as_slice());
            let array = intrlay::create::<adapters::$codec<[u32]>>(&values)?;
            assert_eq!(
                array.as_ref(),
                intrlay::create_from::<Box<[u32]>>(&values)?.as_ref()
            );
            assert_eq!(
                adapters::$codec::<[u32]>::decode(array.as_ref(), 0)?.as_ref(),
                [1, 2, 3]
            );
            assert_eq!(
                intrlay::create_from::<adapters::$codec<[u32]>>(&ZeroedSequence::new(3))?.as_ref(),
                intrlay::create_from::<Vec<u32>>(&[0, 0, 0][..])?.as_ref()
            );
            let text: Owner<str> = Owner::from("shared UTF-8 🦀");
            let string = intrlay::create::<adapters::$codec<str>>(&text)?;
            assert_eq!(
                string.as_ref(),
                intrlay::create_from::<String>(&text)?.as_ref()
            );
            assert_eq!(
                adapters::$codec::<str>::decode(string.as_ref(), 0)?.as_ref(),
                text.as_ref()
            );
            assert_eq!(
                intrlay::create_from::<Codec<u32>>(&BoxInput(42_u32))?.as_ref(),
                intrlay::create::<Codec<u32>>(&shared)?.as_ref()
            );
            Ok(())
        }
    };
}

shared_owner!(rc_occurrences_are_independent, std::rc::Rc, Rc);
shared_owner!(arc_occurrences_are_independent, std::sync::Arc, Arc);

#[test]
fn recursive_shared_owners_remain_tree_codecs() -> Result<(), Box<dyn Error>> {
    use std::rc::Rc;
    #[derive(Debug, PartialEq, Intrlay)]
    struct Node {
        value: u32,
        #[intrlay(with = Vec<adapters::Rc<Node>>)]
        children: Vec<Rc<Node>>,
    }
    let child = Rc::new(Node {
        value: 3,
        children: vec![],
    });
    let value = Node {
        value: 1,
        children: vec![child.clone(), child],
    };
    let bytes = intrlay::create::<Node>(&value)?;
    Node::validate(bytes.as_ref(), 0)?;
    let decoded = Node::decode(bytes.as_ref(), 0)?;
    assert_eq!(decoded, value);
    assert!(!Rc::ptr_eq(&decoded.children[0], &decoded.children[1]));
    Ok(())
}
