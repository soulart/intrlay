//! Container ownership does not change numeric projection or the sequence range.

use std::{error::Error, rc::Rc, sync::Arc};

use intrlay::{Intrlay, schema::Registry};

use super::Fixtures;

#[derive(Intrlay)]
#[intrlay(generate_byte_range_accessors = true)]
struct NumericOwners {
    boxed: Box<[u32]>,
    vector: Vec<u32>,
    #[intrlay(with = intrlay::adapters::Rc<[u32]>)]
    rc: Rc<[u32]>,
    #[intrlay(with = intrlay::adapters::Arc<[u32]>)]
    arc: Arc<[u32]>,
    optional: Option<Box<[u32]>>,
    empty: Box<[u32]>,
    absent: Option<Box<[u32]>>,
    nested: Vec<Box<[u32]>>,
    fixed_slices: [Box<[u32]>; 2],
    fixed: Box<[u32; 2]>,
    matrix: Vec<[f32; 2]>,
    tuple: (Box<str>, Box<[u32]>, u32),
    #[intrlay(project = "flatten")]
    flattened: (u32, [u32; 2]),
    optional_items: Box<[Option<u32>]>,
    #[intrlay(project = "fields")]
    structural: Box<[u32]>,
    #[intrlay(with = intrlay::adapters::Rc<str>)]
    rc_text: Rc<str>,
    #[intrlay(with = intrlay::adapters::Arc<str>)]
    arc_text: Arc<str>,
    #[intrlay(with = intrlay::adapters::Rc<Vec<u32>>)]
    rc_vector: Rc<Vec<u32>>,
}

pub(super) fn collect(
    fixtures: &mut Fixtures,
    registry: &mut Registry,
) -> Result<(), Box<dyn Error>> {
    registry.export::<NumericOwners>("NumericOwners");
    let value = NumericOwners {
        boxed: [11, 12].into(),
        vector: vec![21, 22],
        rc: [31, 32].as_slice().into(),
        arc: [41, 42].as_slice().into(),
        optional: Some([51, 52].into()),
        empty: [].into(),
        absent: None,
        nested: vec![[61, 62].into(), [].into()],
        fixed_slices: [[71, 72].into(), [].into()],
        fixed: Box::new([81, 82]),
        matrix: vec![[1.0, 2.0], [3.0, 4.0]],
        tuple: ("tuple".into(), [91, 92].into(), 93),
        flattened: (101, [102, 103]),
        optional_items: [Some(111), None].into(),
        structural: [121, 122].into(),
        rc_text: "rc".into(),
        arc_text: "arc".into(),
        rc_vector: Rc::new(vec![131, 132]),
    };
    let bytes = intrlay::create::<NumericOwners>(&value)?;
    fixtures.insert("compositions/numeric-owners.bin", bytes.as_ref())?;
    fixtures.insert(
        "compositions/numeric-owners-data.dart",
        format!("const numericOwnerBytes = {:?};\n", bytes.as_ref()),
    )?;
    Ok(())
}

#[test]
fn numeric_slice_owners_share_sequence_representation() -> Result<(), Box<dyn Error>> {
    use intrlay::schema::{Describe, Language, Type};
    use intrlay_codegen::generate;

    #[derive(Intrlay)]
    struct OnlyBoxedSlice {
        samples: Box<[u32]>,
    }

    let mut registry = Registry::new();
    let sequence = <Vec<u32>>::describe(&mut registry);
    assert!(matches!(sequence, Type::Sequence(_)));
    assert_eq!(<Box<[u32]>>::describe(&mut registry), sequence);
    assert_eq!(
        intrlay::adapters::Rc::<[u32]>::describe(&mut registry),
        sequence
    );
    assert_eq!(
        intrlay::adapters::Arc::<[u32]>::describe(&mut registry),
        sequence
    );
    registry.export::<OnlyBoxedSlice>("OnlyBoxedSlice");
    let schema = registry.finish();
    for language in [Language::TypeScript, Language::Python, Language::Dart] {
        let generated = generate(&schema, language)?;
        assert!(!generated.contains("IntrlaySequenceOfU32View"));
    }
    Ok(())
}
