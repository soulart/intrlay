//! Projection helpers and ordinary views of one type have distinct destinations.

use intrlay::{
    Intrlay,
    schema::{Language, Registry, Schema},
};
use intrlay_codegen::generate;

use super::{Fixtures, Point};

#[derive(Intrlay)]
struct Sample {
    value: f32,
    // Valid on the ordinary view; only a projection requests byte-range helpers.
    value_byte_offset: u32,
}

#[derive(Intrlay)]
enum Event {
    Empty,
    Value {
        #[intrlay(project = "flatten")]
        sample: Sample,
    },
}

#[derive(Intrlay)]
#[intrlay(view(generate_setters = false))]
struct Ordinary {
    sample: Sample,
    event: Event,
    point: Point,
}

#[derive(Intrlay)]
#[intrlay(create = false, generate_getters = false, generate_setters = false)]
struct InlineOnly {
    value: f32,
}

#[derive(Intrlay)]
struct Projected {
    #[intrlay(
        project = "flatten",
        view(generate_value_accessors = false, generate_byte_range_accessors = true)
    )]
    flat: Sample,
    #[intrlay(
        project = "flatten",
        view(generate_value_accessors = false, generate_byte_range_accessors = true)
    )]
    nested: [[Sample; 2]; 2],
    #[intrlay(
        project = "indexed_only",
        view(generate_value_accessors = false, generate_byte_range_accessors = true)
    )]
    indexed: Vec<Sample>,
    #[intrlay(
        project = "indexed_only",
        view(generate_value_accessors = false, generate_byte_range_accessors = true)
    )]
    events: Vec<Event>,
    #[intrlay(project = "flatten")]
    flat_point: Point,
    #[intrlay(project = "indexed_only")]
    indexed_points: Vec<Point>,
    #[intrlay(
        project = "flatten",
        generate_getters = true,
        generate_setters = true,
        view(generate_getters = true, generate_setters = true)
    )]
    inline_only: InlineOnly,
}

fn schema(exported_point: bool) -> Schema {
    let mut registry = Registry::new();
    registry.export::<Ordinary>("Ordinary");
    registry.export::<Projected>("Projected");
    registry.export::<InlineOnly>("InlineOnly");
    if exported_point {
        registry.export::<Point>("Point");
    }
    registry.finish()
}

pub(super) fn collect(fixtures: &mut Fixtures) -> Result<(), Box<dyn std::error::Error>> {
    for (name, exported) in [("projection_targets", false), ("projection_exports", true)] {
        for (language, extension) in [
            (Language::TypeScript, "ts"),
            (Language::Python, "py"),
            (Language::Dart, "dart"),
        ] {
            fixtures.insert(
                format!("view-configuration/{name}.{extension}"),
                generate(&schema(exported), language)?,
            )?;
        }
    }
    Ok(())
}

#[test]
fn projection_requests_do_not_expand_the_ordinary_view() -> Result<(), Box<dyn std::error::Error>> {
    for language in [Language::TypeScript, Language::Python, Language::Dart] {
        // Each projection is valid on its own. Sharing the schema must not add
        // helpers to SampleView that collide with its value_byte_offset field.
        generate(&schema(false), language)?;
        generate(&schema(true), language)?;
    }
    let source = generate(&schema(false), Language::TypeScript)?;
    assert!(source.contains("get flatValueByteOffset()"));
    assert!(source.contains("getValueByteOffset(index:"));
    assert!(source.contains("getValueSampleValueByteOffset(index:"));
    assert!(source.contains("set flatPointX("));
    assert!(source.contains("setX(index:"));
    assert!(!source.contains("public set x("));
    assert!(!source.contains("class InlineOnlyView"));
    assert!(source.contains("get inlineOnlyValue()"));
    let exported = generate(&schema(true), Language::TypeScript)?;
    assert!(exported.contains("public set x("));
    Ok(())
}
