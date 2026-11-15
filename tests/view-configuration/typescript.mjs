import assert from "node:assert/strict";
import fs from "node:fs";
import * as read from "../../target/view-configuration/read.js";
import * as shared from "../../target/view-configuration/shared.js";
import * as ranges from "../../target/view-configuration/ranges.js";
import * as elements from "../../target/view-configuration/elements.js";
import * as enabled from "../../target/view-configuration/enabled.js";
import * as filtered from "../../target/view-configuration/filtered.js";
import * as projections from "../../target/view-configuration/projections.js";
import * as defaults from "../../target/view-configuration/defaults.js";
import * as flattened from "../../target/view-configuration/flattened.js";
import * as transparent from "../../target/view-configuration/transparent.js";
import * as merged from "../../target/view-configuration/merged.js";
import * as inputs from "../../target/view-configuration/inputs.js";
import * as omitted from "../../target/view-configuration/omitted.js";
import * as inherited from "../../target/view-configuration/inherited.js";
import * as correlated from "../../target/view-configuration/correlated.js";
import * as correlatedEnabled from "../../target/view-configuration/correlated_enabled.js";
import * as navigation from "../../target/view-configuration/navigation.js";
import * as inactive from "../../target/view-configuration/inactive.js";
import * as helperRequests from "../../target/view-configuration/helper_requests.js";
import * as nestedFlatten from "../../target/view-configuration/nested_flatten.js";
import * as unusedIndexed from "../../target/view-configuration/unused_indexed.js";
import * as usedIndexed from "../../target/view-configuration/used_indexed.js";
import * as numericUses from "../../target/view-configuration/numeric_uses.js";
import * as numericExport from "../../target/view-configuration/numeric_export.js";
import * as projectionTargets from "../../target/view-configuration/projection_targets.js";
import * as projectionExports from "../../target/view-configuration/projection_exports.js";

const bytes = () => new Uint8Array(fs.readFileSync("target/view-configuration/read.bin"));
const sample = new read.ReadFrameView(bytes());
for (const [point, expected] of [
    [sample.position, 1], [sample.optional, 4], [sample.points.get(0), 6],
    [sample.fixed.get(1), 10], [sample.choice.pointPoint, 12],
    [sample.branch.position, 14], [sample.branch.next.position, 16],
]) {
    assert.equal(point.x, expected);
    assert.equal(Object.getOwnPropertyDescriptor(Object.getPrototypeOf(point), "x").set, undefined);
}
assert.equal("setPosition" in sample, false);
assert.equal("set" in sample.points, false);

const combined = new shared.ReadFrameView(bytes());
assert.ok(combined.position instanceof shared.Vec3View);
combined.position.x = 40;
assert.equal(combined.position.x, 40);
assert.equal("setPosition" in combined, false);
combined.optional.y = 50;
assert.equal(combined.optional.y, 50);

const raw = ranges.RangesView.create({ position: { x: 1, y: 2, z: 3 } });
assert.equal("x" in raw.position, false);
assert.equal(raw.position.yByteLength, 4);
assert.equal(raw.storage.f32[raw.position.yByteOffset >>> 2], 2);

const collection = elements.ElementPolicyView.create({ points: [{ x: 1, y: 2 }] });
collection.points.set(0, { x: 7, y: 8 });
assert.equal(collection.points.get(0).x, 7);
assert.equal(Object.getOwnPropertyDescriptor(elements.PointView.prototype, "x").set, undefined);

const restored = enabled.EnabledView.create({ child: { value: 1 } });
restored.child.value = 9;
assert.equal(restored.child.value, 9);
const nested = enabled.EnabledNestedView.create({ container: { child: { value: 3 }, hidden: 7 } });
nested.container.child.value = 12;
assert.equal(nested.container.child.value, 12);
assert.equal("hidden" in nested.container, false);

const local = filtered.LanguagePolicyView.create({ child: { x: 1, y: 2 } });
local.child.x = 4;
assert.equal(local.child.x, 4);
assert.equal(local.child.xByteLength, 4);

const projected = projections.ProjectionsView.create({
    rows: [{ values: new Float32Array([1, 2]) }],
    flat: { x: 3, y: 4 }, numeric: new Float32Array([5, 6]),
});
assert.equal(projected.rows.getValuesComponent(0, 1), 2);
assert.equal(projected.rows.getValuesByteLength(0), 8);
assert.equal("get" in projected.rows, false);
assert.equal("setValuesComponent" in projected.rows, false);
assert.equal(projected.flatX, 3);
assert.equal(Object.getOwnPropertyDescriptor(projections.ProjectionsView.prototype, "flatX").set, undefined);
assert.equal(projected.numeric[1], 6);

const helpers = defaults.SharedHelpersView.create({
    first: { values: [1, 2] }, second: { values: [3, 4] },
});
assert.equal(helpers.first.getValuesComponent(1), 2);
assert.equal(helpers.first.valuesByteLength, 8);
assert.equal(helpers.first.constructor, helpers.second.constructor);
assert.equal("values" in helpers.first, false);
helpers.second.setValuesComponent(1, 5);
assert.equal(helpers.second.getValuesComponent(1), 5);

const flat = flattened.FlattenedRangesView.create({ points: [{ x: 1, y: 2 }, { x: 3, y: 4 }] });
assert.equal("points0X" in flat, false);
assert.equal(flat.points0XByteOffset, 0);
assert.equal(flat.points1YByteLength, 4);
assert.equal(flat.storage.f32[flat.points1YByteOffset >>> 2], 4);

const wrappers = transparent.TransparentUserView.create({
    point: { x: 1, y: 2 }, boxed: { x: 3, y: 4 },
    optional: { x: 5, y: 6 }, items: [{ x: 7, y: 8 }],
});
wrappers.setPoint({ x: 9, y: 10 });
wrappers.setBoxed({ x: 11, y: 12 });
wrappers.setOptional({ x: 13, y: 14 });
wrappers.items.set(0, { x: 15, y: 16 });
assert.deepEqual([wrappers.point.x, wrappers.boxed.x, wrappers.optional.x, wrappers.items.get(0).x], [9, 11, 13, 15]);
assert.equal(Object.getOwnPropertyDescriptor(transparent.PointView.prototype, "x").set, undefined);

const requests = merged.MergedViewsView.create({ a: { point: { x: 1, y: 2 } }, b: { point: { x: 3, y: 4 } } });
assert.equal(requests.a.pointByteLength, 8);
assert.equal("point" in requests.a, false);
assert.equal("point" in requests.b, false);
assert.equal("PointView" in merged, false);

const encoded = inputs.ProjectionInputsView.create({ lanes: new Float32Array([1, 2]), structured: { x: 3, y: 4 } });
assert.deepEqual([...encoded.lanes], [1, 2]);
assert.equal(encoded.structured.x, 3);
assert.equal("pointX" in omitted.OmittedFlattenView.create({ point: { x: 1, y: 2 } }), false);
const inheritedFlat = inherited.InheritedFlattenView.create({ point: { x: 1, y: 2 } });
assert.equal(inheritedFlat.storage.f32[inheritedFlat.pointYByteOffset >>> 2], 2);

const entry = { kind: 1, point: { x: 1, y: 2 }, values: [3, 4] };
const requestsInput = {
    read: [entry], write: [entry], fixedRead: [entry, entry], fixedWrite: [entry, entry], value: entry,
};
const paired = correlated.CollectionRequestsView.create(requestsInput);
for (const collection of [paired.read, paired.write, paired.fixedRead, paired.fixedWrite]) {
    for (const name of ["get", "getTag", "getSamplePointX", "setSamplePointX",
        "getSampleValuesComponent", "setSampleValuesComponent", "getSampleValuesByteOffset"]) {
        assert.equal(name in collection, false, name);
    }
}
assert.equal(paired.read.constructor, paired.write.constructor);
paired.write.set(0, { kind: 0 });
paired.fixedWrite.set(1, { kind: 0 });
paired.value.samplePointX = 5;
assert.equal(paired.value.samplePointX, 5);

const allowed = correlatedEnabled.EnabledCollectionView.create({ entries: [entry] });
const sharedRequests = correlatedEnabled.CollectionRequestsView.create(requestsInput);
assert.equal(sharedRequests.read.constructor, allowed.entries.constructor);
assert.equal(allowed.entries.getTag(0), 1);
allowed.entries.setSamplePointX(0, 7);
assert.equal(allowed.entries.getSamplePointX(0), 7);
allowed.entries.setSampleValuesComponent(0, 1, 8);
assert.equal(allowed.entries.getSampleValuesComponent(0, 1), 8);
assert.equal(allowed.storage.f32[allowed.entries.getSampleValuesByteOffset(0) >>> 2], 3);
assert.equal(allowed.entries.get(0).samplePointX, 7);

const noElements = navigation.ElementNavigationView.create({ omitted: [{ x: 1, y: 2 }], indexed: [{ x: 3, y: 4 }] });
assert.equal(noElements.omitted.constructor, noElements.indexed.constructor);
assert.equal("get" in noElements.indexed, false);
noElements.indexed.setX(0, 9);
assert.equal(noElements.indexed.getX(0), 9);
const flatAndNested = navigation.FlatAndNestedView.create({ points: [{ x: 1, y: 2 }, { x: 3, y: 4 }], ordinary: { x: 5, y: 6 } });
assert.equal(flatAndNested.points1Y, 4);
assert.equal(Object.getOwnPropertyDescriptor(navigation.FlatAndNestedView.prototype, "points0X").set, undefined);
flatAndNested.ordinary.x = 9;
assert.equal(flatAndNested.ordinary.x, 9);
const foldedCollections = navigation.FoldedCollectionsView.create({ read: [{ x: 1, y: 2 }], write: [{ x: 3, y: 4 }] });
assert.equal(foldedCollections.read.constructor, foldedCollections.write.constructor);
foldedCollections.write.set(0, { x: 5, y: 6 });
assert.equal(foldedCollections.write.get(0).x, 5);

const inactiveExports = inactive.ReadPointView.create({ point: { x: 1, y: 2 } });
assert.equal(inactiveExports.point.x, 1);
assert.equal(Object.getOwnPropertyDescriptor(inactive.PointView.prototype, "x").set, undefined);
assert.equal("HiddenPointView" in inactive, false);
assert.equal("HiddenNestedView" in inactive, false);
const createOnly = inactive.CreateOnlyPointView.create({ x: 6, y: 7 });
assert.deepEqual([...createOnly.storage.f32], [6, 7]);

const helperPair = helperRequests.HelperRequestsView.create({
    ranges: { items: [{ values: [1, 2] }] }, components: { items: [{ values: [3, 4] }] },
});
assert.equal(helperPair.ranges.items.constructor, helperPair.components.items.constructor);
assert.equal("getValuesComponent" in helperPair.ranges.items, false);
helperPair.components.items.setValuesComponent(0, 1, 11);
assert.equal(helperPair.storage.f32[(helperPair.components.items.getValuesByteOffset(0) >>> 2) + 1], 11);
const point = { x: 1, y: 2 };
const nestedRanges = nestedFlatten.NestedRangesView.create({
    nested: [[point, point], [point, point]], records: [{ point }, { point }], primitives: [[3, 4], [5, 6]],
    optional: [point, null], numeric: [new Float32Array([7, 8]), new Float32Array([9, 10])],
});
assert.equal(nestedRanges.nested11YByteOffset, 28);
assert.equal(nestedRanges.records1PointYByteOffset, 44);
assert.equal(nestedRanges.primitives11ByteOffset, 60);
assert.equal(nestedRanges.nested11YByteLength, 4);
assert.equal(nestedRanges.optional1ByteOffset, 76);
assert.equal(nestedRanges.optional1ByteLength, 12);
assert.equal(nestedRanges.numeric1ByteOffset, 96);
assert.equal(nestedRanges.numeric1ByteLength, 8);
assert.equal(nestedRanges.storage.f32[nestedRanges.numeric1ByteOffset >>> 2], 9);
assert.equal(nestedRanges.storage.f32[nestedRanges.records1PointYByteOffset >>> 2], 2);
assert.equal(nestedRanges.storage.f32[nestedRanges.primitives11ByteOffset >>> 2], 6);
assert.equal("nested11Y" in nestedRanges, false);
assert.equal("PointView" in nestedFlatten, false);
const nestedComponents = nestedFlatten.NestedComponentsView.create({ items: [{ values: [1, 2] }, { values: [3, 4] }] });
nestedComponents.setItems1ValuesComponent(1, 9);
assert.equal(nestedComponents.getItems1ValuesComponent(1), 9);
assert.equal(nestedComponents.getItems0ValuesComponent(1), 2);
assert.equal("items1Values" in nestedComponents, false);

const hiddenIndexed = unusedIndexed.UnusedIndexedView.create({
    items: [{ point }], points: [point, point], ranges: [{ point }],
    fixedRanges: [{ point }, { point }], wrappedRanges: [{ point }],
});
assert.equal("getPoint" in hiddenIndexed.items, false);
assert.equal("get" in hiddenIndexed.items, false);
assert.equal("setX" in hiddenIndexed.points, false);
const inheritedRanges = unusedIndexed.InheritedRangesView.create({ items: [{ point }] });
for (const collection of [hiddenIndexed.ranges, hiddenIndexed.fixedRanges, hiddenIndexed.wrappedRanges, inheritedRanges.items]) {
    assert.equal("getPoint" in collection, false);
    assert.equal("get" in collection, false);
    assert.equal(collection.getPointByteLength(0), 8);
    assert.equal(collection.storage.f32[collection.getPointByteOffset(0) >>> 2], 1);
}
assert.equal(Object.getOwnPropertyDescriptor(unusedIndexed.PointView.prototype, "x").set, undefined);
const liveIndexed = usedIndexed.UsedIndexedView.create({ items: [{ point }] });
liveIndexed.items.getPoint(0).x = 8;
assert.equal(liveIndexed.items.getPoint(0).x, 8);

const lanes = new Float32Array([3, 4]);
const numericValues = numericUses.NumericUsesView.create({ optional: lanes, boxed: lanes, sequence: [lanes], fixed: [lanes, lanes], wrapped: lanes });
assert.deepEqual([numericValues.optional[1], numericValues.boxed[1], numericValues.sequence.get(0)[1], numericValues.fixed.get(1)[1], numericValues.wrapped[1]], [4, 4, 4, 4, 4]);
assert.equal(Object.getOwnPropertyDescriptor(numericUses.PointView.prototype, "x").set, undefined);
assert.equal("NumericView" in numericUses, false);
const exportedNumeric = numericExport.NumericView.create(lanes);
exportedNumeric.point.x = 7;
assert.equal(exportedNumeric.point.x, 7);
const targetSample = { value: 3, valueByteOffset: 321 };
const targetEvent = { kind: 1, sample: targetSample };
const ordinaryTargets = projectionTargets.OrdinaryView.create({ sample: targetSample, event: targetEvent, point });
assert.equal(ordinaryTargets.sample.valueByteOffset, 321);
assert.equal(ordinaryTargets.event.valueSampleValueByteOffset, 321);
assert.equal("valueByteLength" in ordinaryTargets.sample, false);
assert.equal("valueSampleValueByteLength" in ordinaryTargets.event, false);
assert.equal(Object.getOwnPropertyDescriptor(projectionTargets.PointView.prototype, "x").set, undefined);
const projectedTargets = projectionTargets.ProjectedView.create({
    flat: targetSample, nested: [[targetSample, targetSample], [targetSample, targetSample]],
    indexed: [targetSample], events: [{ kind: 0 }, targetEvent], flatPoint: point, indexedPoints: [point],
    inlineOnly: { value: 4 },
});
assert.equal(projectedTargets.flatValueByteOffset, 0);
assert.equal(projectedTargets.flatValueByteOffsetByteOffset, 4);
assert.equal(projectedTargets.nested11ValueByteOffset, 32);
assert.equal(projectedTargets.storage.f32[projectedTargets.nested11ValueByteOffset >>> 2], 3);
assert.equal(projectedTargets.storage.u32[projectedTargets.indexed.getValueByteOffsetByteOffset(0) >>> 2], 321);
assert.equal(projectedTargets.events.getTag(0), 0);
assert.equal(projectedTargets.events.getTag(1), 1);
assert.equal(projectedTargets.storage.f32[projectedTargets.events.getValueSampleValueByteOffset(1) >>> 2], 3);
projectedTargets.flatPointX = 9;
projectedTargets.indexedPoints.setX(0, 7);
assert.equal(projectedTargets.flatPointX, 9);
assert.equal(projectedTargets.indexedPoints.getX(0), 7);
projectedTargets.inlineOnlyValue = 5;
assert.equal(projectedTargets.inlineOnlyValue, 5);
assert.equal("InlineOnlyView" in projectionTargets, false);
const exportedPoint = projectionExports.PointView.create(point);
exportedPoint.x = 8;
assert.equal(exportedPoint.x, 8);
console.log("child view configuration: TypeScript passed");
