//! Field-specific projections share the canonical encoding graph. Compatible
//! collection APIs combine the operations required by reachable accessors.

use std::collections::{BTreeMap, BTreeSet};

use intrlay_schema::Exposure;

use crate::GenerateError;

use super::{
    AccessorPolicy, AccessorRequest, Id, Kind, Options, TypeGraph, accessor_requests::AccessorHints,
};

// Identical helper families can originate from different uses. Keep those
// requests distinct until reachable collection APIs are deliberately unioned.
type ProjectionContext = (Id, Exposure, AccessorPolicy, Option<Vec<AccessorRequest>>);

/// Context-specific projections and the views actually instantiated by the API.
pub struct ViewPlan {
    pub types: TypeGraph,
    pub emitted: BTreeSet<Id>,
    pub borrowed: BTreeSet<Id>,
}

impl ViewPlan {
    pub(super) fn new(source: &TypeGraph) -> Result<Self, GenerateError> {
        let configured = source.configure_views();
        let source = &configured;
        let mut builder = ProjectionBuilder {
            source,
            plan: source.clone(),
            contexts: source
                .nodes
                .iter()
                .enumerate()
                .map(|(id, node)| {
                    (
                        (
                            id,
                            node.options.exposure,
                            node.options.accessor_policy(),
                            node.options.requests.clone(),
                        ),
                        id,
                    )
                })
                .collect(),
        };
        // Reserve canonical contexts before following recursive child references.
        for id in 0..source.nodes.len() {
            builder.populate(id, id)?;
        }
        let mut types = builder.plan;
        let emitted = types.collect_views();
        let emitted = types.merge_collection_views(emitted);
        let borrowed = types.collect_borrowed_views(&emitted);
        types.document_collections(&emitted);
        Ok(Self {
            types,
            emitted,
            borrowed,
        })
    }
}

impl TypeGraph {
    /// Use the same field-local helper choices when discovering child views and
    /// when constructing their projected collection API.
    pub(super) fn collection_hints(&self, id: Id, hints: &Options) -> Option<AccessorHints> {
        match self.nodes[id].kind {
            Kind::Sequence(element) | Kind::Array { element, .. }
                if self.nodes[element].has_field_payload() =>
            {
                Some(AccessorHints::new(hints))
            }
            _ => None,
        }
    }

    /// Ordinary and indexed collection access need the same storage and return
    /// types. Merge their requested operations instead of emitting API subsets.
    /// Numeric projections and component-helper policies retain distinct views.
    fn merge_collection_views(&mut self, emitted: BTreeSet<Id>) -> BTreeSet<Id> {
        if self.backend.language.has_value_types() {
            return emitted;
        }

        // Only live views contribute. The unused canonical collection's default
        // API must not introduce an element getter into an indexed-only view.
        let mut collections: BTreeMap<(Id, AccessorPolicy), Vec<Id>> = BTreeMap::new();
        for &id in &emitted {
            let node = &self.nodes[id];
            if let Kind::Sequence(element) | Kind::Array { element, .. } = node.kind
                && self.nodes[element].has_field_payload()
                && matches!(
                    node.options.exposure,
                    Exposure::Fields | Exposure::Indexed | Exposure::IndexedOnly
                )
            {
                collections
                    .entry((node.codec, node.options.accessor_policy()))
                    .or_default()
                    .push(id);
            }
        }

        let mut aliases: Vec<_> = (0..self.nodes.len()).collect();
        for ((codec, policy), members) in collections {
            // IDs are ordered, so a live canonical/exported view stays canonical.
            let target = members[0];
            let mut merged = self.nodes[target].clone();
            let mut elements = false;
            let mut indexed = false;
            for id in members {
                let node = &self.nodes[id];
                elements |= node.options.exposure != Exposure::IndexedOnly;
                indexed |= node.options.exposure != Exposure::Fields;
                merged.options.generation.getters |= node.options.generation.getters;
                merged.options.generation.setters |= node.options.generation.setters;
                merged.options.generation.create |= node.options.generation.create;
                merged.options.merge_requests(&node.options);
                aliases[id] = target;
            }
            merged.options.exposure = match (elements, indexed) {
                (true, false) => Exposure::Fields,
                (true, true) => Exposure::Indexed,
                _ => Exposure::IndexedOnly,
            };
            merged.view_name.clone_from(&self.nodes[codec].view_name);
            policy_suffix(
                &mut merged.view_name,
                self.nodes[codec].options.accessor_policy(),
                policy,
            );
            self.nodes[target] = merged;
        }

        // Every operation in the union already existed on a reachable view, so
        // remapping retains the same child-view and encoding dependencies.
        for node in &mut self.nodes {
            match &mut node.kind {
                Kind::Record(fields) => {
                    for field in fields {
                        field.ty = aliases[field.ty];
                    }
                }
                Kind::Enum(variants) => {
                    for field in variants.iter_mut().flat_map(|variant| &mut variant.fields) {
                        field.ty = aliases[field.ty];
                    }
                }
                Kind::Sequence(value)
                | Kind::Array { element: value, .. }
                | Kind::Optional { value, .. }
                | Kind::Boxed(value) => *value = aliases[*value],
                _ => {}
            }
        }
        emitted.into_iter().map(|id| aliases[id]).collect()
    }
}

struct ProjectionBuilder<'a> {
    source: &'a TypeGraph,
    plan: TypeGraph,
    contexts: BTreeMap<ProjectionContext, Id>,
}

impl ProjectionBuilder<'_> {
    fn project(
        &mut self,
        id: Id,
        exposure: Exposure,
        hints: &Options,
    ) -> Result<Id, GenerateError> {
        let source = &self.source.nodes[id];
        // A collection field can opt into helpers without changing another
        // field using the same array type. Non-collection options stay local
        // to their declaration and are consulted by IndexedAccessors directly.
        let mut options = source.options.clone();
        if let Some(helpers) = self.source.collection_hints(id, hints) {
            helpers.apply(&mut options);
            options.override_request_helpers(hints);
        }
        let key = (
            id,
            exposure,
            options.accessor_policy(),
            options.requests.clone(),
        );
        if let Some(&known) = self.contexts.get(&key) {
            return Ok(known);
        }
        let mut node = source.clone();
        node.options = options;
        node.options.exposure = exposure;
        node.options.generation.create = false;
        if exposure != source.options.exposure {
            node.view_name.push_str(match exposure {
                Exposure::Fields => "Fields",
                Exposure::Flatten => "Flatten",
                Exposure::Indexed => "Indexed",
                Exposure::IndexedOnly => "IndexedOnly",
                Exposure::TypedArray(_) => "Numeric",
            });
        }
        policy_suffix(
            &mut node.view_name,
            source.options.accessor_policy(),
            node.options.accessor_policy(),
        );
        let target = self.plan.nodes.len();
        self.contexts.insert(key, target);
        self.plan.nodes.push(node);
        self.populate(id, target)?;
        Ok(target)
    }

    fn populate(&mut self, source: Id, target: Id) -> Result<(), GenerateError> {
        let mut kind = self.source.nodes[source].kind.clone();
        match &mut kind {
            Kind::Record(fields) => {
                let mut flattened = super::flatten::fields(
                    self.source,
                    fields,
                    self.plan.nodes[target].options.exposure == Exposure::Flatten
                        && !self.plan.nodes[target].transparent,
                )?;
                *fields = flattened.fields;
                for field in fields {
                    field.docs = self
                        .source
                        .projected_field_docs(field, &mut flattened.contexts);
                    self.project_field(field, target)?;
                }
                self.plan.nodes[target].docs =
                    flattened.contexts.document(&self.plan.nodes[target].docs);
            }
            Kind::Enum(variants) => {
                for variant in variants {
                    let mut flattened =
                        super::flatten::fields(self.source, &variant.fields, false)?;
                    variant.fields = flattened.fields;
                    for field in &mut variant.fields {
                        field.docs = self
                            .source
                            .projected_field_docs(field, &mut flattened.contexts);
                        self.project_field(field, target)?;
                    }
                    self.plan.nodes[target].docs =
                        flattened.contexts.document(&self.plan.nodes[target].docs);
                }
            }
            _ => {}
        }
        self.plan.nodes[target].kind = kind;
        Ok(())
    }

    fn project_field(&mut self, field: &mut super::Field, owner: Id) -> Result<(), GenerateError> {
        // Container defaults apply just as field-local hints do. A collection's
        // helper API must inherit them when its navigation getter stays available.
        let mut hints = field.options.clone();
        hints.inherit_helpers(&self.plan.nodes[owner].options);
        field.ty = self.project(field.ty, field.options.exposure, &hints)?;
        Ok(())
    }
}

fn policy_suffix(name: &mut String, source: AccessorPolicy, target: AccessorPolicy) {
    for (before, after, enabled, disabled) in [
        (
            source.components,
            target.components,
            "Components",
            "NoComponents",
        ),
        (
            source.byte_ranges,
            target.byte_ranges,
            "ByteRanges",
            "NoByteRanges",
        ),
        (source.values, target.values, "Values", "NoValues"),
    ] {
        if before != after {
            name.push_str(if after { enabled } else { disabled });
        }
    }
}
