//! Keep each use's read/write gates paired with its accessor families.
//!
//! Unioning the gates and families independently can invent an operation: a
//! range-only reader plus an omitted value reader must not enable value reads.

use intrlay_schema::{ApiGeneration, Exposure};

use super::{Field, Node, Options};

/// Field-local helper choices specialize a returned collection without changing
/// its read/write gates or the configuration inherited by nested views.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct AccessorHints {
    components: Option<bool>,
    byte_ranges: Option<bool>,
    values: Option<bool>,
}

impl AccessorHints {
    pub(super) fn new(options: &Options) -> Self {
        Self {
            components: options.component_accessors,
            byte_ranges: options.byte_range_accessors,
            values: options.value_accessors,
        }
    }

    pub(super) fn apply(self, options: &mut Options) {
        options.component_accessors = self.components.or(options.component_accessors);
        options.byte_range_accessors = self.byte_ranges.or(options.byte_range_accessors);
        options.value_accessors = self.values.or(options.value_accessors);
    }
}

/// A resolved use of a type, before its API is shared with other uses.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ViewContextId(pub usize);

/// A request either emits on this view or supplies fields to a containing API.
/// Projection metadata must not add methods to an independently returned view.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum RequestTarget {
    View,
    Projection,
}

/// Preserve the edge to the requested child while compatible views are merged.
/// Indexed and flattened access must combine requests connected by this edge,
/// rather than taking the Cartesian product of the two types' requests.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct RequestScope {
    context: ViewContextId,
    child: Option<ViewContextId>,
    exposure: Exposure,
    target: RequestTarget,
}

/// One inherited request before compatible views combine their emitted APIs.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AccessorRequest {
    pub getters: bool,
    pub setters: bool,
    pub components: Option<bool>,
    pub byte_ranges: Option<bool>,
    pub values: Option<bool>,
    scope: Option<RequestScope>,
}

impl AccessorRequest {
    pub(super) fn new(options: &Options) -> Self {
        Self {
            getters: options.generation.getters,
            setters: options.generation.setters,
            components: options.component_accessors,
            byte_ranges: options.byte_range_accessors,
            values: options.value_accessors,
            scope: None,
        }
    }

    /// Field choices precede the containing declaration's defaults.
    pub fn inherit(mut self, parent: Self) -> Self {
        self.getters &= parent.getters;
        self.setters &= parent.setters;
        self.inherit_helpers(parent);
        self
    }

    /// A child request is relevant only to the use that actually reached it.
    pub fn follows(self, parent: Self) -> bool {
        match (self.scope, parent.scope) {
            (Some(child), Some(parent)) => parent.child == Some(child.context),
            (None, None) => true,
            _ => false,
        }
    }

    pub fn indexed(self) -> bool {
        self.scope
            .is_none_or(|scope| matches!(scope.exposure, Exposure::Indexed | Exposure::IndexedOnly))
    }

    pub fn elements(self) -> bool {
        self.scope
            .is_none_or(|scope| scope.exposure != Exposure::IndexedOnly)
    }

    fn on_view(self) -> bool {
        self.scope
            .is_none_or(|scope| scope.target == RequestTarget::View)
    }

    fn reparent(self, parent: Self) -> Self {
        let mut result = self.inherit(parent);
        if let (Some(scope), Some(parent)) = (&mut result.scope, parent.scope) {
            scope.context = parent.context;
            scope.target = parent.target;
        }
        result
    }

    pub(super) fn inherit_helpers(&mut self, parent: Self) {
        self.components = self.components.or(parent.components);
        self.byte_ranges = self.byte_ranges.or(parent.byte_ranges);
        self.values = self.values.or(parent.values);
    }

    pub fn value_generation(self) -> ApiGeneration {
        ApiGeneration {
            getters: self.getters && self.values.unwrap_or(true),
            setters: self.setters && self.values.unwrap_or(true),
            create: false,
        }
    }

    pub fn generation(self) -> ApiGeneration {
        ApiGeneration {
            getters: self.getters,
            setters: self.setters,
            create: false,
        }
    }
}

impl Options {
    pub fn accessor_requests(&self) -> impl Iterator<Item = AccessorRequest> + '_ {
        self.requests
            .iter()
            .flatten()
            .copied()
            .chain(self.requests.is_none().then(|| AccessorRequest::new(self)))
    }

    pub fn value_generation(&self) -> ApiGeneration {
        self.view_requests().fold(
            ApiGeneration {
                getters: false,
                setters: false,
                create: self.generation.create,
            },
            |mut result, request| {
                let values = request.value_generation();
                result.getters |= values.getters;
                result.setters |= values.setters;
                result
            },
        )
    }

    /// Operations emitted on this view, excluding fields borrowed by a projection.
    pub fn view_requests(&self) -> impl Iterator<Item = AccessorRequest> + '_ {
        self.accessor_requests().filter(|request| request.on_view())
    }

    pub(super) fn capture_requests(
        &mut self,
        parent: Option<&Self>,
        context: ViewContextId,
        exposure: Exposure,
    ) {
        let mut request = AccessorRequest::new(self);
        request.scope = Some(RequestScope {
            context,
            child: None,
            exposure,
            target: RequestTarget::View,
        });
        self.requests = Some(vec![parent.map_or(request, |parent| {
            request.inherit(AccessorRequest::new(parent))
        })]);
    }

    /// After discovering a use's children, retain its requests as projection
    /// metadata. Flattening moves them onto the receiving parent's target;
    /// indexed access selects them through the collection/element context edge.
    pub(super) fn mark_projected_requests(&mut self) {
        for request in self.requests.iter_mut().flatten() {
            if let Some(scope) = &mut request.scope {
                scope.target = RequestTarget::Projection;
            }
        }
    }

    pub(super) fn set_child_context(&mut self, child: ViewContextId) {
        for request in self.requests.iter_mut().flatten() {
            if let Some(scope) = &mut request.scope {
                scope.child = Some(child);
            }
        }
    }

    /// Move an inline child's operations onto its containing view. The original
    /// parent/child pairing survives nested flattening and array expansion.
    pub(super) fn inherit_flattened_requests(&mut self, parent: &Self) {
        if self.requests.is_none() && parent.requests.is_none() {
            return;
        }
        self.requests = Some(
            self.accessor_requests()
                .flat_map(|child| {
                    parent
                        .accessor_requests()
                        .filter(move |&parent| child.follows(parent))
                        .map(move |parent| child.reparent(parent))
                })
                .collect(),
        );
    }

    pub(super) fn merge_requests(&mut self, other: &Self) {
        if self.requests.is_none() {
            self.requests = Some(vec![AccessorRequest::new(self)]);
        }
        if let Some(requests) = &mut self.requests {
            for request in other.accessor_requests() {
                if !requests.contains(&request) {
                    requests.push(request);
                }
            }
        }
    }

    pub(super) fn limit_requests(&mut self, generation: ApiGeneration) {
        for request in self.requests.iter_mut().flatten() {
            request.getters &= generation.getters;
            request.setters &= generation.setters;
        }
    }

    pub(super) fn inherit_request_helpers(&mut self, parent: &Self) {
        // Captured requests already inherit their own owner. Flattening joins
        // matching requests explicitly; merged raw defaults must not leak back.
        if parent.requests.is_some() {
            return;
        }
        let parent = AccessorRequest::new(parent);
        for request in self.requests.iter_mut().flatten() {
            request.inherit_helpers(parent);
        }
    }

    /// Projection-specific collection hints override only the named families.
    pub(super) fn override_request_helpers(&mut self, hints: &Self) {
        if self.requests.is_some() && hints.requests.is_some() {
            // Apply each field's hints only to the child use reached by that
            // field. Its read/write gates control the parent accessor, not the
            // returned collection's API, so only helper choices are overridden.
            self.requests = Some(
                self.accessor_requests()
                    .flat_map(|child| {
                        hints
                            .accessor_requests()
                            .filter(move |&parent| child.follows(parent))
                            .map(move |parent| AccessorRequest {
                                components: parent.components.or(child.components),
                                byte_ranges: parent.byte_ranges.or(child.byte_ranges),
                                values: parent.values.or(child.values),
                                ..child
                            })
                    })
                    .collect(),
            );
            return;
        }
        for request in self.requests.iter_mut().flatten() {
            request.components = hints.component_accessors.or(request.components);
            request.byte_ranges = hints.byte_range_accessors.or(request.byte_ranges);
            request.values = hints.value_accessors.or(request.values);
        }
    }
}

pub fn field_requests(owner: &Node, field: &Field) -> impl Iterator<Item = AccessorRequest> {
    inherited_field_requests(owner, field).filter(|request| request.on_view())
}

fn inherited_field_requests(owner: &Node, field: &Field) -> impl Iterator<Item = AccessorRequest> {
    field.options.accessor_requests().flat_map(move |request| {
        // Captured fields already include their containing declaration's helper
        // defaults. Flattened fields still respect the new owner's read/write gates.
        let captured = field.options.requests.is_some();
        let limited = AccessorRequest {
            getters: request.getters && owner.options.generation.getters,
            setters: request.setters && owner.options.generation.setters,
            ..request
        };
        captured.then_some(limited).into_iter().chain(
            owner
                .options
                .accessor_requests()
                .filter(move |_| !captured)
                .map(move |parent| request.inherit(parent)),
        )
    })
}

pub fn indexed_requests<'a>(
    collection: &'a Node,
    element: &'a Node,
    field: &'a Field,
) -> impl Iterator<Item = AccessorRequest> + 'a {
    let indexed = matches!(
        collection.options.exposure,
        intrlay_schema::Exposure::Indexed | intrlay_schema::Exposure::IndexedOnly
    );
    inherited_field_requests(element, field).flat_map(move |request| {
        collection
            .options
            .view_requests()
            .filter(move |&parent| indexed && parent.indexed() && request.follows(parent))
            .map(move |parent| request.inherit(parent))
    })
}
