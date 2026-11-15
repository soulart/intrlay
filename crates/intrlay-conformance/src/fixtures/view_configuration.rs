//! A shared adapter API combines only the requests of reachable use sites.

mod combinations;
mod projection_targets;
mod reachability;
mod requests;

use intrlay::{
    Intrlay,
    adapters::glam_0_34 as math,
    schema::{Language, Registry, Schema},
};
use intrlay_codegen::generate;

use super::Fixtures;

#[derive(Intrlay)]
struct Branch {
    position: Point,
    next: Option<Box<Self>>,
}

#[derive(Intrlay)]
struct Point {
    x: f32,
    y: f32,
}

#[derive(Intrlay)]
enum Choice {
    Empty,
    Point { point: Point },
}

#[derive(Intrlay)]
#[intrlay(view(generate_setters = false))]
struct ReadFrame {
    #[intrlay(with = math::Vec3, generate_setters = false)]
    position: glam::Vec3,
    optional: Option<Box<Point>>,
    points: Vec<Point>,
    fixed: [Point; 2],
    choice: Choice,
    branch: Branch,
}

#[derive(Intrlay)]
struct WriteFrame {
    #[intrlay(with = math::Vec3, view(generate_setters = true))]
    position: glam::Vec3,
    #[intrlay(view(generate_setters = true))]
    point: Point,
}

#[derive(Intrlay)]
struct Ranges {
    #[intrlay(with = math::Vec3, view(
        generate_value_accessors = false,
        generate_byte_range_accessors = true,
        generate_setters = false,
    ))]
    position: glam::Vec3,
}

#[derive(Intrlay)]
#[intrlay(view(generate_setters = false))]
struct ElementPolicy {
    #[intrlay(view(generate_setters = true, elements(generate_setters = false)))]
    points: Vec<Point>,
}

#[derive(Intrlay)]
#[intrlay(generate_getters = false, generate_setters = false)]
struct Disabled {
    value: u32,
}

#[derive(Intrlay)]
struct Enabled {
    #[intrlay(
        generate_getters = true,
        view(generate_getters = true, generate_setters = true)
    )]
    child: Disabled,
}

#[derive(Intrlay)]
struct Container {
    child: Disabled,
    #[intrlay(generate_getters = false, generate_setters = false)]
    hidden: u32,
}

#[derive(Intrlay)]
struct EnabledNested {
    #[intrlay(view(generate_getters = true, generate_setters = true))]
    container: Option<Box<Container>>,
}

#[derive(Intrlay)]
#[intrlay(view(generate_setters = false))]
struct LanguagePolicy {
    #[intrlay(
        view(generate_byte_range_accessors = true),
        typescript(view(generate_setters = true)),
        python(view(generate_byte_range_accessors = false))
    )]
    child: Point,
}

#[derive(Intrlay)]
struct Lanes {
    values: [f32; 2],
}

#[derive(Intrlay)]
struct Projections {
    #[intrlay(
        project = "indexed_only",
        view(
            generate_value_accessors = false,
            generate_component_accessors = true,
            generate_byte_range_accessors = true,
            generate_setters = false,
        )
    )]
    rows: Vec<Lanes>,
    #[intrlay(project = "flatten", view(generate_setters = false))]
    flat: Point,
    #[intrlay(project = "f32", view(generate_setters = false))]
    numeric: Vec<[f32; 2]>,
}

#[derive(Intrlay)]
struct HiddenFrame {
    #[intrlay(generate_getters = false, generate_setters = false)]
    point: Point,
}

fn schema(shared: bool) -> Schema {
    let mut registry = Registry::new();
    registry.export::<ReadFrame>("ReadFrame");
    if shared {
        registry.export::<WriteFrame>("WriteFrame");
    }
    registry.finish()
}

pub(super) fn collect(fixtures: &mut Fixtures) -> Result<(), Box<dyn std::error::Error>> {
    combinations::collect(fixtures)?;
    projection_targets::collect(fixtures)?;
    reachability::collect(fixtures)?;
    requests::collect(fixtures)?;
    let mut ranges = Registry::new();
    ranges.export::<Ranges>("Ranges");
    let mut elements = Registry::new();
    elements.export::<ElementPolicy>("ElementPolicy");
    let mut enabled = Registry::new();
    enabled.export::<Enabled>("Enabled");
    enabled.export::<EnabledNested>("EnabledNested");
    let mut filtered = Registry::new();
    filtered.export::<LanguagePolicy>("LanguagePolicy");
    let mut projections = Registry::new();
    projections.export::<Projections>("Projections");
    for (name, schema) in [
        ("read", schema(false)),
        ("shared", schema(true)),
        ("ranges", ranges.finish()),
        ("elements", elements.finish()),
        ("enabled", enabled.finish()),
        ("filtered", filtered.finish()),
        ("projections", projections.finish()),
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
    let point = |x| Point { x, y: x + 1.0 };
    let sample = ReadFrame {
        position: glam::Vec3::new(1.0, 2.0, 3.0),
        optional: Some(Box::new(point(4.0))),
        points: vec![point(6.0)],
        fixed: [point(8.0), point(10.0)],
        choice: Choice::Point { point: point(12.0) },
        branch: Branch {
            position: point(14.0),
            next: Some(Box::new(Branch {
                position: point(16.0),
                next: None,
            })),
        },
    };
    let bytes = intrlay::create::<ReadFrame>(&sample)?;
    fixtures.insert("view-configuration/read.bin", bytes.as_ref())?;
    fixtures.insert(
        "view-configuration/data.dart",
        format!("const sampleBytes = {:?};\n", bytes.as_ref()),
    )?;
    Ok(())
}

#[test]
fn hidden_and_explicit_exports_contribute_different_requests()
-> Result<(), Box<dyn std::error::Error>> {
    let mut registry = Registry::new();
    registry.export::<ReadFrame>("ReadFrame");
    registry.export::<HiddenFrame>("HiddenFrame");
    let hidden = generate(&registry.finish(), Language::TypeScript)?;
    assert!(!hidden.contains("public set x("));
    let mut registry = Registry::new();
    registry.export::<ReadFrame>("ReadFrame");
    registry.export::<Point>("Point");
    let explicit = generate(&registry.finish(), Language::TypeScript)?;
    assert!(explicit.contains("public set x("));
    Ok(())
}

#[test]
fn projections_preserve_use_site_accessor_requests() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = Registry::new();
    registry.export::<Projections>("Projections");
    let source = generate(&registry.finish(), Language::TypeScript)?;
    assert!(source.contains("getValuesComponent("));
    assert!(!source.contains("setValuesComponent("));
    assert!(!source.contains("public set flatX("));
    assert!(!source.contains("export class LanesView"));
    assert!(!source.contains("public get(index:"));
    Ok(())
}

#[test]
fn adapter_requests_merge_without_unrequested_defaults() -> Result<(), Box<dyn std::error::Error>> {
    let read = generate(&schema(false), Language::TypeScript)?;
    assert!(read.contains("public get x(): number"));
    assert!(!read.contains("public set x("));
    assert!(!read.contains("setPoint("));
    let shared = generate(&schema(true), Language::TypeScript)?;
    assert_eq!(shared.matches("export class Vec3View ").count(), 1);
    assert_eq!(shared.matches("export class PointView ").count(), 1);
    assert!(shared.contains("public set x("));
    Ok(())
}

#[test]
fn element_configuration_does_not_trim_the_collection_writer()
-> Result<(), Box<dyn std::error::Error>> {
    let mut registry = Registry::new();
    registry.export::<ElementPolicy>("ElementPolicy");
    let source = generate(&registry.finish(), Language::TypeScript)?;
    assert!(source.contains("public set(index: number"));
    assert!(!source.contains("public set x("));
    Ok(())
}
