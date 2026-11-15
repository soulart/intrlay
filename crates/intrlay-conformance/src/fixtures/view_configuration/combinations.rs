//! Independent modules prevent another use from masking an omitted API.

use intrlay::{
    Intrlay,
    schema::{Describe, Language, Registry, Schema},
};
use intrlay_codegen::generate;

use super::{Fixtures, Point};

#[derive(Intrlay)]
#[intrlay(
    generate_component_accessors = true,
    generate_byte_range_accessors = true,
    generate_value_accessors = false
)]
struct HelperDefaults {
    values: [f32; 2],
}

#[derive(Intrlay)]
struct SharedHelpers {
    #[intrlay(generate_value_accessors = true, view(generate_setters = false))]
    first: HelperDefaults,
    #[intrlay(generate_value_accessors = true)]
    second: HelperDefaults,
}

#[derive(Intrlay)]
struct FlattenedRanges {
    #[intrlay(
        project = "flatten",
        view(
            generate_value_accessors = false,
            generate_byte_range_accessors = true,
            generate_setters = false,
        )
    )]
    points: [Point; 2],
}

#[derive(Intrlay)]
#[intrlay(generate_getters = false, generate_setters = false)]
struct OmittedFlatten {
    #[intrlay(project = "flatten", view(generate_setters = false))]
    point: Point,
}

#[derive(Intrlay)]
struct InheritedFlatten {
    #[intrlay(
        project = "flatten",
        generate_byte_range_accessors = true,
        view(generate_setters = false)
    )]
    point: Point,
}

#[derive(Intrlay)]
#[intrlay(transparent)]
struct Wrapped(Point);

#[derive(Intrlay)]
#[intrlay(transparent)]
struct BoxedWrapper(Box<Point>);

#[derive(Intrlay)]
struct TransparentUser {
    #[intrlay(view(generate_setters = false))]
    point: Wrapped,
    #[intrlay(view(generate_setters = false))]
    boxed: BoxedWrapper,
    #[intrlay(view(generate_setters = false))]
    optional: Option<Wrapped>,
    #[intrlay(view(elements(generate_setters = false)))]
    items: Vec<Wrapped>,
}

#[derive(Intrlay)]
struct Item {
    point: Point,
}

#[derive(Intrlay)]
struct ProjectionInputs {
    #[intrlay(project = "f32")]
    lanes: Point,
    structured: Point,
}

#[derive(Intrlay)]
#[intrlay(project = "f32")]
struct NumericRoot([f32; 2]);

#[derive(Intrlay)]
struct MergedViews {
    #[intrlay(view(
        generate_getters = true,
        generate_setters = false,
        generate_value_accessors = false,
        generate_byte_range_accessors = true,
    ))]
    a: Item,
    #[intrlay(view(
        generate_getters = false,
        generate_setters = false,
        generate_value_accessors = true,
    ))]
    b: Item,
}

fn schema<T: Describe>(name: &str) -> Schema {
    let mut registry = Registry::new();
    registry.export::<T>(name);
    registry.finish()
}

pub(super) fn collect(fixtures: &mut Fixtures) -> Result<(), Box<dyn std::error::Error>> {
    let mut inputs = Registry::new();
    inputs.export::<ProjectionInputs>("ProjectionInputs");
    inputs.export::<NumericRoot>("NumericRoot");
    let mut omitted = Registry::new();
    omitted.export::<OmittedFlatten>("OmittedFlatten");
    omitted.export::<Point>("Point");
    for (name, schema) in [
        ("defaults", schema::<SharedHelpers>("SharedHelpers")),
        ("flattened", schema::<FlattenedRanges>("FlattenedRanges")),
        ("omitted", omitted.finish()),
        ("inherited", schema::<InheritedFlatten>("InheritedFlatten")),
        ("transparent", schema::<TransparentUser>("TransparentUser")),
        ("merged", schema::<MergedViews>("MergedViews")),
        ("inputs", inputs.finish()),
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
fn numeric_aliases_require_an_explicit_export() -> Result<(), Box<dyn std::error::Error>> {
    for language in [Language::TypeScript, Language::Python, Language::Dart] {
        let source = generate(&schema::<ProjectionInputs>("ProjectionInputs"), language)?;
        assert!(!source.contains("PointNumericInput"));
        assert!(source.contains("PointInput"));
        let explicit = generate(&schema::<NumericRoot>("NumericRoot"), language)?;
        assert!(explicit.contains("NumericRootInput"));
    }
    Ok(())
}

#[test]
fn shared_helpers_keep_inherited_defaults() -> Result<(), Box<dyn std::error::Error>> {
    let source = generate(
        &schema::<SharedHelpers>("SharedHelpers"),
        Language::TypeScript,
    )?;
    assert!(source.contains("getValuesComponent("));
    assert!(source.contains("setValuesComponent("));
    assert!(source.contains("public get valuesByteOffset()"));
    assert!(!source.contains("public get values()"));
    Ok(())
}

#[test]
fn flatten_traverses_arrays_without_element_getters() -> Result<(), Box<dyn std::error::Error>> {
    let source = generate(
        &schema::<FlattenedRanges>("FlattenedRanges"),
        Language::TypeScript,
    )?;
    assert!(source.contains("public get points0XByteOffset()"));
    assert!(source.contains("public get points1YByteLength()"));
    assert!(!source.contains("public get points0X()"));
    Ok(())
}

#[test]
fn flattened_fields_keep_outer_gates_and_helper_defaults() -> Result<(), Box<dyn std::error::Error>>
{
    let mut registry = Registry::new();
    registry.export::<OmittedFlatten>("OmittedFlatten");
    registry.export::<Point>("Point");
    let omitted = generate(&registry.finish(), Language::TypeScript)?;
    assert!(!omitted.contains("public get pointX()"));
    assert!(!omitted.contains("public set pointX("));
    let inherited = generate(
        &schema::<InheritedFlatten>("InheritedFlatten"),
        Language::TypeScript,
    )?;
    assert!(inherited.contains("public get pointXByteOffset()"));
    Ok(())
}

#[test]
fn transparent_child_omissions_preserve_parent_replacement()
-> Result<(), Box<dyn std::error::Error>> {
    let source = generate(
        &schema::<TransparentUser>("TransparentUser"),
        Language::TypeScript,
    )?;
    for method in [
        "setPoint(",
        "setBoxed(",
        "setOptional(",
        "public set(index:",
    ] {
        assert!(source.contains(method), "missing {method}");
    }
    assert!(!source.contains("public set x("));
    Ok(())
}

#[test]
fn merged_requests_do_not_invent_value_navigation() -> Result<(), Box<dyn std::error::Error>> {
    let source = generate(&schema::<MergedViews>("MergedViews"), Language::TypeScript)?;
    assert!(source.contains("public get pointByteOffset()"));
    assert!(!source.contains("public get point()"));
    assert!(!source.contains("export class PointView"));
    Ok(())
}
