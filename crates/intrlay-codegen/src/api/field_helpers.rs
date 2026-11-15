//! Auxiliary field APIs share one emission policy across ordinary and indexed
//! fields. Byte ranges expose wire storage; components expose numeric lanes.

use crate::plan::{
    AccessorRequest, ByteRange, Field, Node, TypeGraph, field_requests, indexed_requests,
};

#[derive(Default)]
pub struct FieldHelpers {
    pub component_getter: Option<String>,
    pub component_setter: Option<String>,
    pub byte_offset: Option<String>,
    pub byte_length: Option<String>,
}

impl FieldHelpers {
    pub fn range_docs(plan: &TypeGraph, field: &Field, indexed: bool, offset: bool) -> String {
        let docs = if indexed {
            TypeGraph::indexed_field_docs(field, false)
        } else {
            field.docs.clone()
        };
        let note = match (plan.byte_range(field), offset) {
            (ByteRange::Sequence { .. }, true) => {
                "Byte offset of the first array element, relative to the first byte of Storage. Follows the sequence pointer without creating a view. An inactive enum variant returns the absent result."
            }
            (ByteRange::Sequence { .. }, false) => {
                "Byte length of the contiguous element region: count times wire stride, including element padding. Indirect element payloads are outside this range. An inactive enum variant returns the absent result."
            }
            (ByteRange::Inline { .. }, true) => {
                "Byte offset of the field's inline wire representation, relative to the first byte of Storage. Does not follow pointers or test variant tags or optional presence."
            }
            (ByteRange::Inline { .. }, false) => {
                "Byte length of the field's inline wire representation, including padding. Referenced payloads are outside this range."
            }
        };
        crate::documentation::with_note(&docs, note)
    }

    pub fn component_docs(field: &Field, indexed: bool) -> String {
        if indexed {
            TypeGraph::indexed_field_docs(field, true)
        } else {
            crate::documentation::with_note(
                &field.docs,
                "The component index addresses scalar lanes, including padding, and must be within the numeric projection's length.",
            )
        }
    }

    pub fn field(plan: &TypeGraph, owner: &Node, field: &Field, name: &str) -> Self {
        Self::new(plan, field, name, field_requests(owner, field), false)
    }

    pub fn indexed(
        plan: &TypeGraph,
        owner: &Node,
        element: &Node,
        field: &Field,
        name: &str,
    ) -> Self {
        Self::new(
            plan,
            field,
            name,
            indexed_requests(owner, element, field),
            true,
        )
    }

    fn new(
        plan: &TypeGraph,
        field: &Field,
        name: &str,
        requests: impl Iterator<Item = AccessorRequest>,
        indexed: bool,
    ) -> Self {
        if !plan.backend.language.supports_accessor_generation() {
            return Self::default();
        }
        let mut read_components = false;
        let mut write_components = false;
        let mut ranges = false;
        for request in requests {
            read_components |= request.components.unwrap_or(false) && request.getters;
            write_components |= request.components.unwrap_or(false) && request.setters;
            ranges |= request.byte_ranges.unwrap_or(false) && request.getters;
        }
        let numeric =
            (read_components || write_components) && plan.component_scalar(field).is_some();
        let range_name = |suffix| {
            if indexed {
                plan.names.method(&format!("get_{name}_{suffix}"), false)
            } else {
                plan.names.property(&format!("{name}_{suffix}"), true)
            }
        };
        Self {
            component_getter: (numeric && read_components)
                .then(|| plan.names.method(&format!("get_{name}_component"), false)),
            component_setter: (numeric && write_components)
                .then(|| plan.names.method(&format!("set_{name}_component"), false)),
            byte_offset: ranges.then(|| range_name("byte_offset")),
            byte_length: ranges.then(|| range_name("byte_length")),
        }
    }

    pub fn declarations(&self, read_only: bool) -> impl Iterator<Item = (&str, &'static str)> {
        [
            (self.component_getter.as_deref(), "numeric component getter"),
            (
                self.component_setter.as_deref().filter(|_| !read_only),
                "numeric component setter",
            ),
            (self.byte_offset.as_deref(), "field byte offset"),
            (self.byte_length.as_deref(), "field byte length"),
        ]
        .into_iter()
        .filter_map(|(name, role)| name.map(|name| (name, role)))
    }
}
