#[derive(wire::Intrlay)]
#[intrlay(generate_getters = false, generate_setters = false)]
struct Child { value: u32 }

#[derive(wire::Intrlay)]
struct Parent {
    #[intrlay(generate_setters = false)]
    child: Child,
    #[intrlay(generate_getters = false)]
    scalar: u32,
}

#[derive(wire::Intrlay)]
#[intrlay(view(generate_setters = false))]
struct Configured {
    #[intrlay(view(elements(generate_getters = false)),
        typescript(view(elements(generate_getters = true))))]
    children: Vec<Child>,
}

fn main() {
    let mut view = ParentView::create(&Parent { child: Child { value: 1 }, scalar: 2 }).unwrap();
    view.child_mut().value_mut().set(3);
    view.scalar_mut().set(4);
    assert_eq!(view.child().value(), 3);
    assert_eq!(view.scalar(), 4);
    let mut registry = wire::schema::Registry::default();
    registry.export::<Parent>("Parent");
    let schema = registry.finish();
    let child = schema.definitions.values().find(|definition| definition.name == "Child").unwrap();
    for language in [wire::schema::Language::Rust, wire::schema::Language::Cpp, wire::schema::Language::CSharp] {
        assert!(!child.targets.contains_key(&language));
    }
    for language in [wire::schema::Language::TypeScript, wire::schema::Language::Python, wire::schema::Language::Dart] {
        assert_eq!(child.targets[&language].generate_getters, Some(false));
        assert_eq!(child.targets[&language].generate_setters, Some(false));
    }
    let mut registry = wire::schema::Registry::default();
    registry.export::<Configured>("Configured");
    let schema = registry.finish();
    let configured = schema.definitions.values().find(|definition| definition.name == "Configured").unwrap();
    assert_eq!(configured.targets[&wire::schema::Language::TypeScript].view.as_ref().unwrap().generate_setters, Some(false));
    let wire::schema::DefinitionKind::Record(fields) = &configured.kind else { panic!() };
    for language in [wire::schema::Language::TypeScript, wire::schema::Language::Python, wire::schema::Language::Dart] {
        let options = fields[0].targets[&language].view.as_ref().unwrap().elements.as_ref().unwrap();
        assert_eq!(options.generate_getters, Some(language == wire::schema::Language::TypeScript));
    }
}
