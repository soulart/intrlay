//! A shared view exposes the union of actual operations, not combinations of
//! read/write gates taken from unrelated uses or inactive exports.

use intrlay::{
    Intrlay,
    schema::{Language, Registry, Schema},
};
use intrlay_codegen::generate;

use super::{Fixtures, Point};

#[derive(Intrlay)]
enum Entry {
    Empty,
    Sample {
        #[intrlay(project = "flatten")]
        point: Point,
        values: [f32; 2],
    },
}

#[derive(Intrlay)]
struct CollectionRequests {
    #[intrlay(
        project = "indexed_only",
        view(
            generate_getters = true,
            generate_setters = false,
            generate_component_accessors = true,
            generate_byte_range_accessors = true,
            elements(generate_getters = false, generate_setters = true),
        )
    )]
    read: Vec<Entry>,
    #[intrlay(
        project = "indexed_only",
        view(
            generate_getters = false,
            generate_setters = true,
            generate_component_accessors = true,
            generate_byte_range_accessors = true,
            elements(generate_getters = true, generate_setters = false),
        )
    )]
    write: Vec<Entry>,
    #[intrlay(
        project = "indexed_only",
        view(
            generate_getters = true,
            generate_setters = false,
            generate_component_accessors = true,
            generate_byte_range_accessors = true,
            elements(generate_getters = false, generate_setters = true),
        )
    )]
    fixed_read: [Entry; 2],
    #[intrlay(
        project = "indexed_only",
        view(
            generate_getters = false,
            generate_setters = true,
            generate_component_accessors = true,
            generate_byte_range_accessors = true,
            elements(generate_getters = true, generate_setters = false),
        )
    )]
    fixed_write: [Entry; 2],
    // Its complete entry view must not enable indexed APIs on the collections.
    value: Entry,
}

#[derive(Intrlay)]
struct EnabledCollection {
    #[intrlay(
        project = "indexed",
        view(
            generate_getters = true,
            generate_setters = true,
            generate_component_accessors = true,
            generate_byte_range_accessors = true,
        )
    )]
    entries: Vec<Entry>,
}

#[derive(Intrlay)]
struct ElementNavigation {
    #[intrlay(view(generate_getters = false, generate_setters = false))]
    omitted: Vec<Point>,
    #[intrlay(project = "indexed_only")]
    indexed: Vec<Point>,
}

#[derive(Intrlay)]
struct FlatAndNested {
    #[intrlay(project = "flatten", view(generate_setters = false))]
    points: [Point; 2],
    ordinary: Point,
}

#[derive(Intrlay)]
#[intrlay(transparent)]
struct WrappedPoint(Point);

#[derive(Intrlay)]
struct FoldedCollections {
    #[intrlay(view(generate_setters = false))]
    read: Vec<WrappedPoint>,
    #[intrlay(view(generate_setters = true))]
    write: Vec<WrappedPoint>,
}

#[derive(Intrlay)]
struct Lanes {
    values: [f32; 2],
}

#[derive(Intrlay)]
struct Rows {
    #[intrlay(project = "indexed_only")]
    items: Vec<Lanes>,
}

#[derive(Intrlay)]
struct HelperRequests {
    #[intrlay(view(
        generate_getters = true,
        generate_setters = false,
        generate_value_accessors = false,
        generate_byte_range_accessors = true,
    ))]
    ranges: Rows,
    #[intrlay(view(
        generate_getters = false,
        generate_setters = true,
        generate_value_accessors = false,
        generate_component_accessors = true,
    ))]
    components: Rows,
}

#[derive(Intrlay)]
struct ReadPoint {
    #[intrlay(view(generate_setters = false))]
    point: Point,
}

#[derive(Intrlay)]
#[intrlay(
    transparent,
    create = false,
    generate_getters = false,
    generate_setters = false
)]
struct HiddenPoint(Point);

#[derive(Intrlay)]
#[intrlay(
    transparent,
    create = false,
    generate_getters = false,
    generate_setters = false
)]
struct HiddenNested(Option<Box<Point>>);

#[derive(Intrlay)]
#[intrlay(transparent, generate_getters = false, generate_setters = false)]
struct CreateOnlyPoint(Point);

fn collections(enabled: bool) -> Schema {
    let mut registry = Registry::new();
    registry.export::<CollectionRequests>("CollectionRequests");
    if enabled {
        registry.export::<EnabledCollection>("EnabledCollection");
    }
    registry.finish()
}

fn inactive_exports() -> Schema {
    let mut registry = Registry::new();
    registry.export::<ReadPoint>("ReadPoint");
    registry.export::<HiddenPoint>("HiddenPoint");
    registry.export::<HiddenNested>("HiddenNested");
    registry.export::<CreateOnlyPoint>("CreateOnlyPoint");
    registry.finish()
}

pub(super) fn collect(fixtures: &mut Fixtures) -> Result<(), Box<dyn std::error::Error>> {
    let mut navigation = Registry::new();
    navigation.export::<ElementNavigation>("ElementNavigation");
    navigation.export::<FlatAndNested>("FlatAndNested");
    navigation.export::<FoldedCollections>("FoldedCollections");
    let mut helpers = Registry::new();
    helpers.export::<HelperRequests>("HelperRequests");
    for (name, schema) in [
        ("correlated", collections(false)),
        ("correlated_enabled", collections(true)),
        ("navigation", navigation.finish()),
        ("inactive", inactive_exports()),
        ("helper_requests", helpers.finish()),
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
fn indexed_apis_combine_only_connected_requests() -> Result<(), Box<dyn std::error::Error>> {
    let omitted = generate(&collections(false), Language::TypeScript)?;
    for method in [
        "getTag(index:",
        "getSamplePointX(index:",
        "setSamplePointX(index:",
        "getSampleValuesComponent(index:",
        "setSampleValuesComponent(index:",
        "getSampleValuesByteOffset(index:",
    ] {
        assert!(!omitted.contains(method), "unrequested API: {method}");
    }
    let enabled = generate(&collections(true), Language::TypeScript)?;
    for method in [
        "getTag(index:",
        "getSamplePointX(index:",
        "setSamplePointX(index:",
        "getSampleValuesComponent(index:",
        "setSampleValuesComponent(index:",
        "getSampleValuesByteOffset(index:",
    ] {
        assert!(enabled.contains(method), "missing requested API: {method}");
    }
    Ok(())
}

#[test]
fn inactive_exports_do_not_supply_child_requests() -> Result<(), Box<dyn std::error::Error>> {
    let source = generate(&inactive_exports(), Language::TypeScript)?;
    assert!(!source.contains("class HiddenPointView"));
    assert!(!source.contains("class HiddenNestedView"));
    assert!(source.contains("class CreateOnlyPointView"));
    assert!(!source.contains("public set x("));
    assert!(!source.contains("public set y("));
    Ok(())
}

#[test]
fn inherited_projection_hints_keep_each_request() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = Registry::new();
    registry.export::<HelperRequests>("HelperRequests");
    let source = generate(&registry.finish(), Language::TypeScript)?;
    assert!(source.contains("getValuesByteOffset(index:"));
    assert!(source.contains("setValuesComponent(index:"));
    assert!(!source.contains("getValuesComponent(index:"));
    Ok(())
}
