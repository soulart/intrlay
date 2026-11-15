//! Inline expansion and indexed field access follow their effective parent
//! operations. Numeric lanes do not request the underlying record's child views.

use intrlay::{
    Intrlay,
    schema::{Language, Registry, Schema},
};
use intrlay_codegen::generate;

use super::{Fixtures, Point};

#[derive(Intrlay)]
struct Item {
    point: Point,
}

#[derive(Intrlay)]
struct NestedRanges {
    #[intrlay(
        project = "flatten",
        view(
            generate_value_accessors = false,
            generate_byte_range_accessors = true,
            generate_setters = false,
        )
    )]
    nested: [[Point; 2]; 2],
    #[intrlay(
        project = "flatten",
        view(
            generate_value_accessors = false,
            generate_byte_range_accessors = true,
            generate_setters = false,
        )
    )]
    records: [Item; 2],
    #[intrlay(
        project = "flatten",
        view(
            generate_value_accessors = false,
            generate_byte_range_accessors = true,
            generate_setters = false,
        )
    )]
    primitives: [[f32; 2]; 2],
    #[intrlay(
        project = "flatten",
        view(
            generate_value_accessors = false,
            generate_byte_range_accessors = true,
            generate_setters = false,
        )
    )]
    optional: [Option<Point>; 2],
    #[intrlay(
        project = "flatten",
        view(
            generate_value_accessors = false,
            generate_byte_range_accessors = true,
            generate_setters = false,
        )
    )]
    numeric: [Numeric; 2],
}

#[derive(Intrlay)]
struct NumericLeaf {
    // This explicit projection terminates inherited flattening at the array.
    #[intrlay(project = "f32")]
    values: [f32; 2],
}

#[derive(Intrlay)]
struct NestedComponents {
    #[intrlay(
        project = "flatten",
        view(generate_value_accessors = false, generate_component_accessors = true,)
    )]
    items: [NumericLeaf; 2],
}

#[derive(Intrlay)]
struct ReadPoint {
    #[intrlay(view(generate_setters = false))]
    point: Point,
}

#[derive(Intrlay)]
#[intrlay(transparent)]
struct RangeItems(
    #[intrlay(
        project = "indexed_only",
        generate_value_accessors = false,
        generate_byte_range_accessors = true
    )]
    Vec<Item>,
);

#[derive(Intrlay)]
struct UnusedIndexed {
    #[intrlay(
        project = "indexed_only",
        view(
            generate_getters = true,
            generate_setters = false,
            elements(generate_getters = false, generate_setters = true),
        )
    )]
    items: Vec<Item>,
    #[intrlay(
        project = "indexed_only",
        view(
            generate_getters = true,
            generate_setters = false,
            elements(generate_getters = false, generate_setters = true),
        )
    )]
    points: [Point; 2],
    #[intrlay(
        project = "indexed_only",
        generate_value_accessors = false,
        generate_byte_range_accessors = true
    )]
    ranges: Vec<Item>,
    #[intrlay(
        project = "indexed_only",
        generate_value_accessors = false,
        generate_byte_range_accessors = true
    )]
    fixed_ranges: [Item; 2],
    wrapped_ranges: RangeItems,
}

#[derive(Intrlay)]
#[intrlay(generate_value_accessors = false, generate_byte_range_accessors = true)]
struct InheritedRanges {
    #[intrlay(project = "indexed_only")]
    items: Vec<Item>,
}

#[derive(Intrlay)]
struct UsedIndexed {
    #[intrlay(project = "indexed_only")]
    items: Vec<Item>,
}

#[derive(Intrlay)]
#[intrlay(project = "f32")]
struct Numeric {
    point: Point,
}

#[derive(Intrlay)]
#[intrlay(transparent)]
struct WrappedNumeric(Numeric);

#[derive(Intrlay)]
struct NumericUses {
    optional: Option<Numeric>,
    boxed: Box<Numeric>,
    sequence: Vec<Numeric>,
    fixed: [Numeric; 2],
    wrapped: WrappedNumeric,
}

fn indexed(used: bool) -> Schema {
    let mut registry = Registry::new();
    registry.export::<ReadPoint>("ReadPoint");
    registry.export::<UnusedIndexed>("UnusedIndexed");
    registry.export::<InheritedRanges>("InheritedRanges");
    if used {
        registry.export::<UsedIndexed>("UsedIndexed");
    }
    registry.finish()
}

fn numeric(exported: bool) -> Schema {
    let mut registry = Registry::new();
    registry.export::<ReadPoint>("ReadPoint");
    registry.export::<NumericUses>("NumericUses");
    if exported {
        registry.export::<Numeric>("Numeric");
    }
    registry.finish()
}

fn flattened() -> Schema {
    let mut registry = Registry::new();
    registry.export::<NestedRanges>("NestedRanges");
    registry.export::<NestedComponents>("NestedComponents");
    registry.finish()
}

pub(super) fn collect(fixtures: &mut Fixtures) -> Result<(), Box<dyn std::error::Error>> {
    for (name, schema) in [
        ("nested_flatten", flattened()),
        ("unused_indexed", indexed(false)),
        ("used_indexed", indexed(true)),
        ("numeric_uses", numeric(false)),
        ("numeric_export", numeric(true)),
    ] {
        for (language, extension) in [
            (Language::TypeScript, "ts"),
            (Language::Python, "py"),
            (Language::Dart, "dart"),
        ] {
            fixtures.insert(
                format!("view-configuration/{name}.{extension}"),
                generate(&schema, language)?,
            )?;
        }
    }
    Ok(())
}

#[test]
fn flatten_keeps_nested_paths_without_returning_element_views()
-> Result<(), Box<dyn std::error::Error>> {
    let source = generate(&flattened(), Language::TypeScript)?;
    assert!(source.contains("get nested11YByteOffset()"));
    assert!(source.contains("get records1PointYByteOffset()"));
    assert!(source.contains("get primitives11ByteOffset()"));
    assert!(source.contains("get optional1ByteOffset()"));
    assert!(source.contains("get numeric1ByteLength()"));
    assert!(source.contains("getItems1ValuesComponent("));
    assert!(source.contains("setItems1ValuesComponent("));
    assert!(!source.contains("get nested11Y()"));
    assert!(!source.contains("class PointView"));
    assert!(!source.contains("class ItemView"));
    Ok(())
}

#[test]
fn indexed_children_require_a_reachable_operation() -> Result<(), Box<dyn std::error::Error>> {
    let hidden = generate(&indexed(false), Language::TypeScript)?;
    assert!(!hidden.contains("public set x("));
    assert!(!hidden.contains("getPoint(index:"));
    let exposed = generate(&indexed(true), Language::TypeScript)?;
    assert!(exposed.contains("public set x("));
    assert!(exposed.contains("getPoint(index:"));
    Ok(())
}

#[test]
fn numeric_values_and_exports_request_different_children() -> Result<(), Box<dyn std::error::Error>>
{
    let borrowed = generate(&numeric(false), Language::TypeScript)?;
    assert!(!borrowed.contains("public set x("));
    assert!(!borrowed.contains("class NumericView"));
    let exported = generate(&numeric(true), Language::TypeScript)?;
    assert!(exported.contains("public set x("));
    assert!(exported.contains("class NumericView"));
    Ok(())
}
