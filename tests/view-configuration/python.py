"""Use-site defaults reach adapters, nested containers, and shared child views."""
from pathlib import Path
from array import array
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "target/view-configuration"))
import read
import shared
import ranges
import elements
import enabled
import filtered
import projections
import defaults
import flattened
import transparent
import merged
import inputs
import omitted
import inherited
import correlated
import correlated_enabled
import navigation
import inactive
import helper_requests
import nested_flatten
import unused_indexed
import used_indexed
import numeric_uses
import numeric_export
import projection_targets
import projection_exports


def encoded():
    return bytearray((ROOT / "target/view-configuration/read.bin").read_bytes())


sample = read.ReadFrameView(encoded())
for point, expected in [
    (sample.position, 1), (sample.optional, 4), (sample.points[0], 6),
    (sample.fixed[1], 10), (sample.choice.point_point, 12),
    (sample.branch.position, 14), (sample.branch.next.position, 16),
]:
    assert point.x == expected
    assert isinstance(type(point).x, property) and type(point).x.fset is None
assert not hasattr(sample, "set_position")
assert not hasattr(sample.points, "__setitem__")

combined = shared.ReadFrameView(encoded())
assert isinstance(combined.position, shared.Vec3View)
combined.position.x = 40
assert combined.position.x == 40
assert not hasattr(combined, "set_position")
combined.optional.y = 50
assert combined.optional.y == 50

raw = ranges.RangesView.create({"position": {"x": 1, "y": 2, "z": 3}})
assert not hasattr(raw.position, "x")
assert raw.position.y_byte_length == 4
assert raw.storage.f32[raw.position.y_byte_offset // 4] == 2

collection = elements.ElementPolicyView.create({"points": [{"x": 1, "y": 2}]})
collection.points[0] = {"x": 7, "y": 8}
assert collection.points[0].x == 7
assert elements.PointView.x.fset is None

restored = enabled.EnabledView.create({"child": {"value": 1}})
restored.child.value = 9
assert restored.child.value == 9
nested = enabled.EnabledNestedView.create({"container": {"child": {"value": 3}, "hidden": 7}})
nested.container.child.value = 12
assert nested.container.child.value == 12
assert not hasattr(nested.container, "hidden")

local = filtered.LanguagePolicyView.create({"child": {"x": 1, "y": 2}})
assert type(local.child).x.fset is None
assert not hasattr(local.child, "x_byte_offset")

projected = projections.ProjectionsView.create({
    "rows": [{"values": [1, 2]}], "flat": {"x": 3, "y": 4},
    "numeric": memoryview(array("f", [5, 6])),
})
assert projected.rows.get_values_component(0, 1) == 2
assert projected.rows.get_values_byte_length(0) == 8
assert not hasattr(projected.rows, "__getitem__")
assert not hasattr(projected.rows, "set_values_component")
assert projected.flat_x == 3
assert projections.ProjectionsView.flat_x.fset is None
assert projected.numeric[1] == 6

helpers = defaults.SharedHelpersView.create({
    "first": {"values": [1, 2]}, "second": {"values": [3, 4]},
})
assert helpers.first.get_values_component(1) == 2
assert helpers.first.values_byte_length == 8
assert type(helpers.first) is type(helpers.second)
assert not hasattr(helpers.first, "values")
helpers.second.set_values_component(1, 5)
assert helpers.second.get_values_component(1) == 5

flat = flattened.FlattenedRangesView.create({"points": [{"x": 1, "y": 2}, {"x": 3, "y": 4}]})
assert not hasattr(flat, "points_0_x")
assert flat.points_0_x_byte_offset == 0
assert flat.points_1_y_byte_length == 4
assert flat.storage.f32[flat.points_1_y_byte_offset // 4] == 4

wrappers = transparent.TransparentUserView.create({
    "point": {"x": 1, "y": 2}, "boxed": {"x": 3, "y": 4},
    "optional": {"x": 5, "y": 6}, "items": [{"x": 7, "y": 8}],
})
wrappers.set_point({"x": 9, "y": 10})
wrappers.set_boxed({"x": 11, "y": 12})
wrappers.set_optional({"x": 13, "y": 14})
wrappers.items[0] = {"x": 15, "y": 16}
assert [wrappers.point.x, wrappers.boxed.x, wrappers.optional.x, wrappers.items[0].x] == [9, 11, 13, 15]
assert transparent.PointView.x.fset is None

requests = merged.MergedViewsView.create({"a": {"point": {"x": 1, "y": 2}}, "b": {"point": {"x": 3, "y": 4}}})
assert requests.a.point_byte_length == 8
assert not hasattr(requests.a, "point") and not hasattr(requests.b, "point")
assert not hasattr(merged, "PointView")

encoded = inputs.ProjectionInputsView.create({"lanes": memoryview(array("f", [1, 2])), "structured": {"x": 3, "y": 4}})
assert list(encoded.lanes) == [1, 2] and encoded.structured.x == 3
assert not hasattr(omitted.OmittedFlattenView.create({"point": {"x": 1, "y": 2}}), "point_x")
inherited_flat = inherited.InheritedFlattenView.create({"point": {"x": 1, "y": 2}})
assert inherited_flat.storage.f32[inherited_flat.point_y_byte_offset // 4] == 2

entry = {"kind": correlated.EntryKind.Sample, "point": {"x": 1, "y": 2}, "values": [3, 4]}
requests_input = {
    "read": [entry], "write": [entry], "fixed_read": [entry, entry], "fixed_write": [entry, entry], "value": entry,
}
paired = correlated.CollectionRequestsView.create(requests_input)
for collection in [paired.read, paired.write, paired.fixed_read, paired.fixed_write]:
    for name in ["__getitem__", "get_tag", "get_sample_point_x", "set_sample_point_x",
                 "get_sample_values_component", "set_sample_values_component", "get_sample_values_byte_offset"]:
        assert not hasattr(collection, name), name
assert type(paired.read) is type(paired.write)
paired.write[0] = {"kind": correlated.EntryKind.Empty}
paired.fixed_write[1] = {"kind": correlated.EntryKind.Empty}
paired.value.sample_point_x = 5
assert paired.value.sample_point_x == 5

allowed = correlated_enabled.EnabledCollectionView.create({"entries": [entry]})
shared_requests = correlated_enabled.CollectionRequestsView.create(requests_input)
assert type(shared_requests.read) is type(allowed.entries)
assert allowed.entries.get_tag(0) == correlated_enabled.EntryKind.Sample
allowed.entries.set_sample_point_x(0, 7)
assert allowed.entries.get_sample_point_x(0) == 7
allowed.entries.set_sample_values_component(0, 1, 8)
assert allowed.entries.get_sample_values_component(0, 1) == 8
assert allowed.storage.f32[allowed.entries.get_sample_values_byte_offset(0) // 4] == 3
assert allowed.entries[0].sample_point_x == 7

no_elements = navigation.ElementNavigationView.create({"omitted": [{"x": 1, "y": 2}], "indexed": [{"x": 3, "y": 4}]})
assert type(no_elements.omitted) is type(no_elements.indexed)
assert not hasattr(no_elements.indexed, "__getitem__")
no_elements.indexed.set_x(0, 9)
assert no_elements.indexed.get_x(0) == 9
flat_and_nested = navigation.FlatAndNestedView.create({"points": [{"x": 1, "y": 2}, {"x": 3, "y": 4}], "ordinary": {"x": 5, "y": 6}})
assert flat_and_nested.points_1_y == 4
assert navigation.FlatAndNestedView.points_0_x.fset is None
flat_and_nested.ordinary.x = 9
assert flat_and_nested.ordinary.x == 9
folded_collections = navigation.FoldedCollectionsView.create({"read": [{"x": 1, "y": 2}], "write": [{"x": 3, "y": 4}]})
assert type(folded_collections.read) is type(folded_collections.write)
folded_collections.write[0] = {"x": 5, "y": 6}
assert folded_collections.write[0].x == 5

inactive_exports = inactive.ReadPointView.create({"point": {"x": 1, "y": 2}})
assert inactive_exports.point.x == 1
assert inactive.PointView.x.fset is None
assert not hasattr(inactive, "HiddenPointView")
assert not hasattr(inactive, "HiddenNestedView")
create_only = inactive.CreateOnlyPointView.create({"x": 6, "y": 7})
assert list(create_only.storage.f32) == [6, 7]

helper_pair = helper_requests.HelperRequestsView.create({
    "ranges": {"items": [{"values": [1, 2]}]}, "components": {"items": [{"values": [3, 4]}]},
})
assert type(helper_pair.ranges.items) is type(helper_pair.components.items)
assert not hasattr(helper_pair.ranges.items, "get_values_component")
helper_pair.components.items.set_values_component(0, 1, 11)
assert helper_pair.storage.f32[helper_pair.components.items.get_values_byte_offset(0) // 4 + 1] == 11
point = {"x": 1, "y": 2}
nested_ranges = nested_flatten.NestedRangesView.create({
    "nested": [[point, point], [point, point]], "records": [{"point": point}, {"point": point}], "primitives": [[3, 4], [5, 6]],
    "optional": [point, None], "numeric": [memoryview(array("f", [7, 8])), memoryview(array("f", [9, 10]))],
})
assert nested_ranges.nested_1_1_y_byte_offset == 28
assert nested_ranges.records_1_point_y_byte_offset == 44
assert nested_ranges.primitives_1_1_byte_offset == 60
assert nested_ranges.nested_1_1_y_byte_length == 4
assert nested_ranges.optional_1_byte_offset == 76
assert nested_ranges.optional_1_byte_length == 12
assert nested_ranges.numeric_1_byte_offset == 96
assert nested_ranges.numeric_1_byte_length == 8
assert nested_ranges.storage.f32[nested_ranges.numeric_1_byte_offset // 4] == 9
assert nested_ranges.storage.f32[nested_ranges.records_1_point_y_byte_offset // 4] == 2
assert nested_ranges.storage.f32[nested_ranges.primitives_1_1_byte_offset // 4] == 6
assert not hasattr(nested_ranges, "nested_1_1_y")
assert not hasattr(nested_flatten, "PointView")
nested_components = nested_flatten.NestedComponentsView.create({"items": [
    {"values": memoryview(array("f", [1, 2]))}, {"values": memoryview(array("f", [3, 4]))},
]})
nested_components.set_items_1_values_component(1, 9)
assert nested_components.get_items_1_values_component(1) == 9
assert nested_components.get_items_0_values_component(1) == 2
assert not hasattr(nested_components, "items_1_values")

hidden_indexed = unused_indexed.UnusedIndexedView.create({
    "items": [{"point": point}], "points": [point, point], "ranges": [{"point": point}],
    "fixed_ranges": [{"point": point}, {"point": point}], "wrapped_ranges": [{"point": point}],
})
assert not hasattr(hidden_indexed.items, "get_point")
assert not hasattr(hidden_indexed.items, "__getitem__")
assert not hasattr(hidden_indexed.points, "set_x")
inherited_ranges = unused_indexed.InheritedRangesView.create({"items": [{"point": point}]})
for collection in [hidden_indexed.ranges, hidden_indexed.fixed_ranges, hidden_indexed.wrapped_ranges, inherited_ranges.items]:
    assert not hasattr(collection, "get_point")
    assert not hasattr(collection, "__getitem__")
    assert collection.get_point_byte_length(0) == 8
    assert collection.storage.f32[collection.get_point_byte_offset(0) // 4] == 1
assert unused_indexed.PointView.x.fset is None
live_indexed = used_indexed.UsedIndexedView.create({"items": [{"point": point}]})
live_indexed.items.get_point(0).x = 8
assert live_indexed.items.get_point(0).x == 8

lanes = memoryview(array("f", [3, 4]))
numeric_values = numeric_uses.NumericUsesView.create({"optional": lanes, "boxed": lanes, "sequence": [lanes], "fixed": [lanes, lanes], "wrapped": lanes})
assert [numeric_values.optional[1], numeric_values.boxed[1], numeric_values.sequence[0][1], numeric_values.fixed[1][1], numeric_values.wrapped[1]] == [4, 4, 4, 4, 4]
assert numeric_uses.PointView.x.fset is None
assert not hasattr(numeric_uses, "NumericView")
exported_numeric = numeric_export.NumericView.create(lanes)
exported_numeric.point.x = 7
assert exported_numeric.point.x == 7
target_sample = {"value": 3, "value_byte_offset": 321}
target_event = {"kind": projection_targets.EventKind.Value, "sample": target_sample}
ordinary_targets = projection_targets.OrdinaryView.create({"sample": target_sample, "event": target_event, "point": point})
assert ordinary_targets.sample.value_byte_offset == 321
assert ordinary_targets.event.value_sample_value_byte_offset == 321
assert not hasattr(ordinary_targets.sample, "value_byte_length")
assert not hasattr(ordinary_targets.event, "value_sample_value_byte_length")
assert projection_targets.PointView.x.fset is None
projected_targets = projection_targets.ProjectedView.create({
    "flat": target_sample, "nested": [[target_sample, target_sample], [target_sample, target_sample]],
    "indexed": [target_sample], "events": [{"kind": projection_targets.EventKind.Empty}, target_event],
    "flat_point": point, "indexed_points": [point],
    "inline_only": {"value": 4},
})
assert projected_targets.flat_value_byte_offset == 0
assert projected_targets.flat_value_byte_offset_byte_offset == 4
assert projected_targets.nested_1_1_value_byte_offset == 32
assert projected_targets.storage.f32[projected_targets.nested_1_1_value_byte_offset // 4] == 3
assert projected_targets.storage.u32[projected_targets.indexed.get_value_byte_offset_byte_offset(0) // 4] == 321
assert projected_targets.events.get_tag(0) == projection_targets.EventKind.Empty
assert projected_targets.events.get_tag(1) == projection_targets.EventKind.Value
assert projected_targets.storage.f32[projected_targets.events.get_value_sample_value_byte_offset(1) // 4] == 3
projected_targets.flat_point_x = 9
projected_targets.indexed_points.set_x(0, 7)
assert projected_targets.flat_point_x == 9
assert projected_targets.indexed_points.get_x(0) == 7
projected_targets.inline_only_value = 5
assert projected_targets.inline_only_value == 5
assert not hasattr(projection_targets, "InlineOnlyView")
exported_point = projection_exports.PointView.create(point)
exported_point.x = 8
assert exported_point.x == 8
print("child view configuration: Python passed")
