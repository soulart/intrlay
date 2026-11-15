//! Resolve child-view requests without duplicating a type's encoding or API.
//!
//! A declaration supplies defaults, while a field carries the configuration of
//! that particular use. Traverse reachable uses before merging their requests;
//! seeding every named type would make its defaults defeat explicit omissions.

use std::collections::{BTreeMap, btree_map::Entry};

use intrlay_schema::{Exposure, ViewOptions};

use crate::api::{Accessor, CollectionAccessors};

use super::{
    AccessorRequest, Id, Kind, Node, Options, TypeGraph,
    accessor_requests::{AccessorHints, ViewContextId},
    flatten,
};

/// Why a type contributes requests. Inline operations keep their enclosing
/// gates; returning a value starts that value's independently configured API.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ViewAccess {
    Export,
    Value,
    Flatten(AccessorRequest),
    Indexed(AccessorRequest),
    /// An inline leaf supplies its own access policy but exposes no child view.
    Leaf(AccessorRequest),
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ViewUse {
    ty: Id,
    exposure: Exposure,
    configuration: ViewOptions,
    hints: AccessorHints,
    access: ViewAccess,
}

#[derive(Default)]
struct ViewUses {
    contexts: BTreeMap<ViewUse, ViewContextId>,
    pending: Vec<(ViewUse, ViewContextId)>,
}

impl ViewUses {
    fn register(&mut self, value: ViewUse) -> ViewContextId {
        let context = ViewContextId(self.contexts.len());
        match self.contexts.entry(value) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                self.pending.push((entry.key().clone(), context));
                entry.insert(context);
                context
            }
        }
    }
}

impl TypeGraph {
    pub(super) fn configure_views(&self) -> Self {
        if self.backend.language.has_value_types()
            || !self.nodes.iter().any(|node| {
                node.options.view != ViewOptions::default()
                    || node
                        .fields()
                        .any(|(field, _)| field.options.view != ViewOptions::default())
            })
        {
            return self.clone();
        }

        let mut uses = ViewUses::default();
        for &id in self.exports.values() {
            if self.nodes[id].has_view_api() {
                uses.register(ViewUse {
                    ty: id,
                    exposure: self.nodes[id].options.exposure,
                    configuration: ViewOptions::default(),
                    hints: AccessorHints::default(),
                    access: ViewAccess::Export,
                });
            }
        }
        let mut requested: Vec<Option<Node>> = vec![None; self.nodes.len()];

        while let Some((usage, context)) = uses.pending.pop() {
            let ViewUse {
                ty: id,
                exposure,
                configuration,
                hints,
                access,
            } = usage;
            // Returned numeric lanes do not instantiate the record's view. An
            // explicit export still has its own construction and field APIs.
            if access == ViewAccess::Value && matches!(exposure, Exposure::TypedArray(_)) {
                continue;
            }
            // Recursive records revisit a finite set of explicit option values.
            // Element scopes are consumed only when crossing a collection.
            let mut node = self.nodes[id].clone();
            apply(&mut node.options, &configuration);
            hints.apply(&mut node.options);
            if let ViewAccess::Flatten(parent)
            | ViewAccess::Indexed(parent)
            | ViewAccess::Leaf(parent) = access
            {
                inherit_inline(&mut node.options, parent);
            }
            for field in node.fields_mut() {
                let mut options = configuration.clone();
                options.override_with(&field.declared_accessors);
                apply(&mut field.options, &options);
            }
            let owner = node.options.clone();
            for field in node.fields_mut() {
                if matches!(access, ViewAccess::Leaf(_)) {
                    field.options.requests = Some(Vec::new());
                } else {
                    field
                        .options
                        .capture_requests(Some(&owner), context, exposure);
                }
            }
            node.options.capture_requests(None, context, exposure);

            // Resolve each use and its outgoing edges before sharing requests.
            // Projection affects navigation, not the canonical representation.
            node.options.exposure = exposure;
            let mut children = self.nodes[id].options.view.clone();
            children.override_with(&configuration);
            self.enqueue_views(&mut node, &children, access, &mut uses);
            node.options.exposure = self.nodes[id].options.exposure;

            // Discovery evaluates one use in isolation. Before sharing it, keep
            // fields used by containing APIs separate from this type's own API.
            if !matches!(access, ViewAccess::Export | ViewAccess::Value) {
                node.options.mark_projected_requests();
                for field in node.fields_mut() {
                    field.options.mark_projected_requests();
                }
            }

            if let Some(previous) = &mut requested[id] {
                merge(&mut previous.options, &node.options);
                for (target, (other, _)) in previous.fields_mut().zip(node.fields()) {
                    merge(&mut target.options, &other.options);
                }
            } else {
                requested[id] = Some(node);
            }
        }

        let mut result = self.clone();
        for (node, requested) in result.nodes.iter_mut().zip(requested) {
            if let Some(configured) = requested {
                *node = configured;
            }
        }
        result
    }

    fn enqueue_views(
        &self,
        node: &mut Node,
        children: &ViewOptions,
        access: ViewAccess,
        uses: &mut ViewUses,
    ) {
        if matches!(access, ViewAccess::Leaf(_)) {
            return;
        }
        // Transparent declarations expose their value directly, just like Box
        // and Option. Their own accessor policy must not stop that navigation.
        if access == ViewAccess::Value && self.folded_node(node).is_some() {
            let owner = node.options.clone();
            if let Some(field) = node.fields_mut().next() {
                let mut configuration = children.clone();
                configuration.override_with(&field.options.view);
                let mut hints = field.options.clone();
                hints.inherit_helpers(&owner);
                let child = uses.register(ViewUse {
                    ty: field.ty,
                    exposure: field.options.exposure,
                    configuration,
                    hints: self.collection_hints(field.ty, &hints).unwrap_or_default(),
                    access: ViewAccess::Value,
                });
                field.options.set_child_context(child);
            }
            return;
        }
        match node.kind {
            Kind::Optional { value, .. } | Kind::Boxed(value) => {
                let generation = node.options.value_generation();
                if access == ViewAccess::Value || generation.getters || generation.setters {
                    let child = uses.register(ViewUse {
                        ty: value,
                        exposure: self.nodes[value].options.exposure,
                        configuration: children.clone(),
                        hints: AccessorHints::default(),
                        access: ViewAccess::Value,
                    });
                    node.options.set_child_context(child);
                }
            }
            Kind::Sequence(element) | Kind::Array { element, .. } => {
                if let Some(usage) = self.collection_use(node, element, children, access) {
                    let child = uses.register(usage);
                    node.options.set_child_context(child);
                }
            }
            Kind::Record(_) | Kind::Enum(_) => {
                self.enqueue_fields(node, children, access, uses);
            }
            Kind::Scalar(_) | Kind::String | Kind::Unit => {}
        }
    }

    fn collection_use(
        &self,
        node: &Node,
        element: Id,
        children: &ViewOptions,
        access: ViewAccess,
    ) -> Option<ViewUse> {
        let options = &self.nodes[element].options;
        let configuration = children.for_elements();
        let parent = AccessorRequest::new(&node.options);
        let (exposure, access) = if matches!(access, ViewAccess::Flatten(_)) {
            if flatten::expands(
                self,
                element,
                options.exposure,
                options.explicit_exposure,
                true,
            ) {
                (Exposure::Flatten, ViewAccess::Flatten(parent))
            } else {
                let mut leaf = options.clone();
                apply(&mut leaf, &configuration);
                inherit_inline(&mut leaf, parent);
                let generation = leaf.value_generation();
                let returns_view = !matches!(options.exposure, Exposure::TypedArray(_))
                    && (generation.getters || generation.setters)
                    && self.borrows_content(element);
                (
                    options.exposure,
                    if returns_view {
                        ViewAccess::Value
                    } else {
                        ViewAccess::Leaf(parent)
                    },
                )
            }
        } else if CollectionAccessors::new(self, node, element).getter {
            // Element views expose their own API. Indexed access adds no new
            // child navigation when the same use already returns those views.
            (options.exposure, ViewAccess::Value)
        } else if (parent.getters || parent.setters)
            && matches!(
                node.options.exposure,
                Exposure::Indexed | Exposure::IndexedOnly
            )
        {
            (options.exposure, ViewAccess::Indexed(parent))
        } else {
            return None;
        };
        Some(ViewUse {
            ty: element,
            exposure,
            configuration,
            hints: AccessorHints::default(),
            access,
        })
    }

    fn enqueue_fields(
        &self,
        node: &mut Node,
        children: &ViewOptions,
        access: ViewAccess,
        uses: &mut ViewUses,
    ) {
        let mut contexts = Vec::new();
        let inherited_flatten = matches!(access, ViewAccess::Flatten(_))
            || (node.options.exposure == Exposure::Flatten && !node.transparent);
        for (field, variant) in node.fields() {
            let accessor = Accessor::field(self, node, field, variant);
            let generation = super::field_generation(node, field);
            let flattened = (generation.getters || generation.setters)
                && flatten::expands(
                    self,
                    field.ty,
                    field.options.exposure,
                    field.options.explicit_exposure,
                    inherited_flatten,
                );
            if flattened
                || (accessor.getter && !matches!(field.options.exposure, Exposure::TypedArray(_)))
            {
                let mut configuration = children.clone();
                configuration.override_with(&field.options.view);
                let mut hints = field.options.clone();
                hints.inherit_helpers(&node.options);
                contexts.push(Some(uses.register(ViewUse {
                    ty: field.ty,
                    exposure: if flattened {
                        Exposure::Flatten
                    } else {
                        field.options.exposure
                    },
                    configuration,
                    hints: if flattened {
                        AccessorHints::default()
                    } else {
                        self.collection_hints(field.ty, &hints).unwrap_or_default()
                    },
                    access: if flattened {
                        ViewAccess::Flatten(
                            AccessorRequest::new(&field.options)
                                .inherit(AccessorRequest::new(&node.options)),
                        )
                    } else {
                        ViewAccess::Value
                    },
                })));
            } else {
                contexts.push(None);
            }
        }
        for (field, context) in node.fields_mut().zip(contexts) {
            if let Some(context) = context {
                field.options.set_child_context(context);
            }
        }
    }
}

/// Operations projected onto a parent remain bounded by that parent's gates.
/// This is independent of configuration inheritance, which can override defaults.
fn inherit_inline(options: &mut Options, parent: AccessorRequest) {
    options
        .replacement_setters
        .get_or_insert(options.generation.setters);
    options.generation.getters &= parent.getters;
    options.generation.setters &= parent.setters;
    options.component_accessors = options.component_accessors.or(parent.components);
    options.byte_range_accessors = options.byte_range_accessors.or(parent.byte_ranges);
    options.value_accessors = options.value_accessors.or(parent.values);
}

fn apply(options: &mut Options, configuration: &ViewOptions) {
    if let Some(getters) = configuration.generate_getters {
        options.generation.getters = getters;
    }
    if let Some(setters) = configuration.generate_setters {
        options.replacement_setters = Some(
            options
                .replacement_setters
                .unwrap_or(options.generation.setters)
                || setters,
        );
        options.generation.setters = setters;
    }
    options.component_accessors = configuration
        .generate_component_accessors
        .or(options.component_accessors);
    options.byte_range_accessors = configuration
        .generate_byte_range_accessors
        .or(options.byte_range_accessors);
    options.value_accessors = configuration
        .generate_value_accessors
        .or(options.value_accessors);
}

fn merge(target: &mut Options, other: &Options) {
    target.merge_requests(other);
    target.replacement_setters = Some(
        target
            .replacement_setters
            .unwrap_or(target.generation.setters)
            || other
                .replacement_setters
                .unwrap_or(other.generation.setters),
    );
    target.generation.getters |= other.generation.getters;
    target.generation.setters |= other.generation.setters;
    // Preserve inheritance for families that neither use explicitly selected.
    target.component_accessors = union(target.component_accessors, other.component_accessors);
    target.byte_range_accessors = union(target.byte_range_accessors, other.byte_range_accessors);
    target.value_accessors = union(target.value_accessors, other.value_accessors);
}

fn union(left: Option<bool>, right: Option<bool>) -> Option<bool> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left || right),
        (Some(true), _) | (_, Some(true)) => Some(true),
        _ => None,
    }
}
