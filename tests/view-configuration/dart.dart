import 'dart:typed_data';
import 'package:intrlay/intrlay.dart' as intrlay;
import '../../target/view-configuration/read.dart' as read;
import '../../target/view-configuration/shared.dart' as shared;
import '../../target/view-configuration/ranges.dart' as ranges;
import '../../target/view-configuration/elements.dart' as elements;
import '../../target/view-configuration/enabled.dart' as enabled;
import '../../target/view-configuration/filtered.dart' as filtered;
import '../../target/view-configuration/projections.dart' as projections;
import '../../target/view-configuration/defaults.dart' as defaults;
import '../../target/view-configuration/flattened.dart' as flattened;
import '../../target/view-configuration/transparent.dart' as transparent;
import '../../target/view-configuration/merged.dart' as merged;
import '../../target/view-configuration/inputs.dart' as inputs;
import '../../target/view-configuration/omitted.dart' as omitted;
import '../../target/view-configuration/inherited.dart' as inherited;
import '../../target/view-configuration/correlated.dart' as correlated;
import '../../target/view-configuration/correlated_enabled.dart' as correlatedEnabled;
import '../../target/view-configuration/navigation.dart' as navigation;
import '../../target/view-configuration/inactive.dart' as inactive;
import '../../target/view-configuration/helper_requests.dart' as helperRequests;
import '../../target/view-configuration/nested_flatten.dart' as nestedFlatten;
import '../../target/view-configuration/unused_indexed.dart' as unusedIndexed;
import '../../target/view-configuration/used_indexed.dart' as usedIndexed;
import '../../target/view-configuration/numeric_uses.dart' as numericUses;
import '../../target/view-configuration/numeric_export.dart' as numericExport;
import '../../target/view-configuration/projection_targets.dart' as projectionTargets;
import '../../target/view-configuration/projection_exports.dart' as projectionExports;
import '../../target/view-configuration/data.dart';

void check(bool value) {
  if (!value) throw StateError('child view configuration regression');
}

void absent(void Function() action) {
  try {
    action();
  } on NoSuchMethodError {
    return;
  }
  throw StateError('unrequested member was emitted');
}

void main() {
  final sample = read.ReadFrameView(Uint8List.fromList(sampleBytes));
  final points = <dynamic>[
    sample.position, sample.optional!, sample.points[0], sample.fixed[1],
    sample.choice.pointPoint!, sample.branch.position, sample.branch.next!.position,
  ];
  const expected = [1, 4, 6, 10, 12, 14, 16];
  for (var i = 0; i < points.length; ++i) {
    check(points[i].x == expected[i]);
    absent(() { points[i].x = 99.0; });
  }
  absent(() { (sample as dynamic).setPosition(read.Vec3Input(x: 1, y: 2, z: 3)); });
  absent(() { (sample.points as dynamic)[0] = read.PointInput(x: 1, y: 2); });

  final combined = shared.ReadFrameView(Uint8List.fromList(sampleBytes));
  combined.position.x = 40;
  check(combined.position.x == 40);
  combined.optional!.y = 50;
  check(combined.optional!.y == 50);

  final raw = ranges.RangesView.create(ranges.RangesInput(
    position: ranges.Vec3Input(x: 1, y: 2, z: 3),
  ));
  absent(() { (raw.position as dynamic).x; });
  check(raw.position.yByteLength == 4);
  check(raw.storage.data.getFloat32(raw.position.yByteOffset, Endian.little) == 2);

  final collection = elements.ElementPolicyView.create(elements.ElementPolicyInput(
    points: intrlay.SequenceInput.from([elements.PointInput(x: 1, y: 2)]),
  ));
  collection.points[0] = elements.PointInput(x: 7, y: 8);
  check(collection.points[0].x == 7);
  absent(() { (collection.points[0] as dynamic).x = 20.0; });

  final restored = enabled.EnabledView.create(enabled.EnabledInput(
    child: enabled.DisabledInput(value: 1),
  ));
  restored.child.value = 9;
  check(restored.child.value == 9);
  final nested = enabled.EnabledNestedView.create(enabled.EnabledNestedInput(
    container: enabled.ContainerInput(child: enabled.DisabledInput(value: 3), hidden: 7),
  ));
  nested.container!.child.value = 12;
  check(nested.container!.child.value == 12);
  absent(() { (nested.container as dynamic).hidden; });

  final local = filtered.LanguagePolicyView.create(filtered.LanguagePolicyInput(
    child: filtered.PointInput(x: 1, y: 2),
  ));
  absent(() { (local.child as dynamic).x = 4.0; });
  check(local.child.xByteLength == 4);

  final projected = projections.ProjectionsView.create(projections.ProjectionsInput(
    rows: intrlay.SequenceInput.from([
      projections.LanesInput(values: intrlay.FixedArrayInput.from([1, 2])),
    ]),
    flat: projections.PointInput(x: 3, y: 4),
    numeric: intrlay.SequenceInput.from(Float32List.fromList([5, 6])),
  ));
  check(projected.rows.getValuesComponent(0, 1) == 2);
  check(projected.rows.getValuesByteLength(0) == 8);
  absent(() { (projected.rows as dynamic)[0]; });
  absent(() { (projected.rows as dynamic).setValuesComponent(0, 0, 5.0); });
  check(projected.flatX == 3);
  absent(() { (projected as dynamic).flatX = 8.0; });
  check(projected.numeric[1] == 6);

  final helpers = defaults.SharedHelpersView.create(defaults.SharedHelpersInput(
    first: defaults.HelperDefaultsInput(values: intrlay.FixedArrayInput.from([1, 2])),
    second: defaults.HelperDefaultsInput(values: intrlay.FixedArrayInput.from([3, 4])),
  ));
  check(helpers.first.getValuesComponent(1) == 2);
  check(helpers.first.valuesByteLength == 8);
  check(helpers.first.runtimeType == helpers.second.runtimeType);
  absent(() { (helpers.first as dynamic).values; });
  helpers.second.setValuesComponent(1, 5);
  check(helpers.second.getValuesComponent(1) == 5);

  final flat = flattened.FlattenedRangesView.create(flattened.FlattenedRangesInput(
    points: intrlay.FixedArrayInput.from([
      flattened.PointInput(x: 1, y: 2), flattened.PointInput(x: 3, y: 4),
    ]),
  ));
  absent(() { (flat as dynamic).points0X; });
  check(flat.points0XByteOffset == 0 && flat.points1YByteLength == 4);
  check(flat.storage.data.getFloat32(flat.points1YByteOffset, Endian.little) == 4);

  final wrappers = transparent.TransparentUserView.create(transparent.TransparentUserInput(
    point: transparent.PointInput(x: 1, y: 2),
    boxed: transparent.PointInput(x: 3, y: 4),
    optional: transparent.PointInput(x: 5, y: 6),
    items: intrlay.SequenceInput.from([transparent.PointInput(x: 7, y: 8)]),
  ));
  wrappers.setPoint(transparent.PointInput(x: 9, y: 10));
  wrappers.setBoxed(transparent.PointInput(x: 11, y: 12));
  wrappers.setOptional(transparent.PointInput(x: 13, y: 14));
  wrappers.items[0] = transparent.PointInput(x: 15, y: 16);
  check(wrappers.point.x == 9 && wrappers.boxed.x == 11 && wrappers.optional!.x == 13 && wrappers.items[0].x == 15);
  absent(() { (wrappers.point as dynamic).x = 20.0; });

  final requests = merged.MergedViewsView.create(merged.MergedViewsInput(
    a: merged.ItemInput(point: merged.PointInput(x: 1, y: 2)),
    b: merged.ItemInput(point: merged.PointInput(x: 3, y: 4)),
  ));
  check(requests.a.pointByteLength == 8);
  absent(() { (requests.a as dynamic).point; });
  absent(() { (requests.b as dynamic).point; });
  final encoded = inputs.ProjectionInputsView.create(inputs.ProjectionInputsInput(
    lanes: Float32List.fromList([1, 2]), structured: inputs.PointInput(x: 3, y: 4),
  ));
  check(encoded.lanes[1] == 2 && encoded.structured.x == 3);
  final omittedFlat = omitted.OmittedFlattenView.create(omitted.OmittedFlattenInput(point: omitted.PointInput(x: 1, y: 2)));
  absent(() { (omittedFlat as dynamic).pointX; });
  final inheritedFlat = inherited.InheritedFlattenView.create(inherited.InheritedFlattenInput(point: inherited.PointInput(x: 1, y: 2)));
  check(inheritedFlat.storage.data.getFloat32(inheritedFlat.pointYByteOffset, Endian.little) == 2);

  final entry = correlated.EntrySampleInput(
    point: correlated.PointInput(x: 1, y: 2), values: intrlay.FixedArrayInput.from([3, 4]),
  );
  final paired = correlated.CollectionRequestsView.create(correlated.CollectionRequestsInput(
    read: [entry], write: [entry], fixedRead: [entry, entry], fixedWrite: [entry, entry], value: entry,
  ));
  for (final collection in <dynamic>[paired.read, paired.write, paired.fixedRead, paired.fixedWrite]) {
    absent(() { collection[0]; });
    absent(() { collection.getTag(0); });
    absent(() { collection.getSamplePointX(0); });
    absent(() { collection.setSamplePointX(0, 5.0); });
    absent(() { collection.getSampleValuesComponent(0, 0); });
    absent(() { collection.setSampleValuesComponent(0, 0, 5.0); });
    absent(() { collection.getSampleValuesByteOffset(0); });
  }
  check(paired.read.runtimeType == paired.write.runtimeType);
  paired.write[0] = correlated.EntryEmptyInput();
  paired.fixedWrite[1] = correlated.EntryEmptyInput();
  paired.value.setSamplePointX(5);
  check(paired.value.samplePointX == 5);

  final enabledEntry = correlatedEnabled.EntrySampleInput(
    point: correlatedEnabled.PointInput(x: 1, y: 2), values: intrlay.FixedArrayInput.from([3, 4]),
  );
  final allowed = correlatedEnabled.EnabledCollectionView.create(correlatedEnabled.EnabledCollectionInput(entries: [enabledEntry]));
  final sharedRequests = correlatedEnabled.CollectionRequestsView.create(correlatedEnabled.CollectionRequestsInput(
    read: [enabledEntry], write: [enabledEntry], fixedRead: [enabledEntry, enabledEntry],
    fixedWrite: [enabledEntry, enabledEntry], value: enabledEntry,
  ));
  check(sharedRequests.read.runtimeType == allowed.entries.runtimeType);
  check(allowed.entries.getTag(0) == 1);
  allowed.entries.setSamplePointX(0, 7);
  check(allowed.entries.getSamplePointX(0) == 7);
  allowed.entries.setSampleValuesComponent(0, 1, 8);
  check(allowed.entries.getSampleValuesComponent(0, 1) == 8);
  check(allowed.storage.data.getFloat32(allowed.entries.getSampleValuesByteOffset(0), Endian.little) == 3);
  check(allowed.entries[0].samplePointX == 7);

  final noElements = navigation.ElementNavigationView.create(navigation.ElementNavigationInput(
    omitted: intrlay.SequenceInput.from([navigation.PointInput(x: 1, y: 2)]),
    indexed: intrlay.SequenceInput.from([navigation.PointInput(x: 3, y: 4)]),
  ));
  check(noElements.omitted.runtimeType == noElements.indexed.runtimeType);
  absent(() { (noElements.indexed as dynamic)[0]; });
  noElements.indexed.setX(0, 9);
  check(noElements.indexed.getX(0) == 9);
  final flatAndNested = navigation.FlatAndNestedView.create(navigation.FlatAndNestedInput(
    points: intrlay.FixedArrayInput.from([navigation.PointInput(x: 1, y: 2), navigation.PointInput(x: 3, y: 4)]),
    ordinary: navigation.PointInput(x: 5, y: 6),
  ));
  check(flatAndNested.points1Y == 4);
  absent(() { (flatAndNested as dynamic).points0X = 8.0; });
  flatAndNested.ordinary.x = 9;
  check(flatAndNested.ordinary.x == 9);
  final foldedCollections = navigation.FoldedCollectionsView.create(navigation.FoldedCollectionsInput(
    read: intrlay.SequenceInput.from([navigation.PointInput(x: 1, y: 2)]),
    write: intrlay.SequenceInput.from([navigation.PointInput(x: 3, y: 4)]),
  ));
  check(foldedCollections.read.runtimeType == foldedCollections.write.runtimeType);
  foldedCollections.write[0] = navigation.PointInput(x: 5, y: 6);
  check(foldedCollections.write[0].x == 5);

  final inactiveExports = inactive.ReadPointView.create(inactive.ReadPointInput(point: inactive.PointInput(x: 1, y: 2)));
  check(inactiveExports.point.x == 1);
  absent(() { (inactiveExports.point as dynamic).x = 5.0; });
  final createOnly = inactive.CreateOnlyPointView.create(inactive.PointInput(x: 6, y: 7));
  check(createOnly.storage.data.getFloat32(0, Endian.little) == 6);
  check(createOnly.storage.data.getFloat32(4, Endian.little) == 7);

  final helperPair = helperRequests.HelperRequestsView.create(helperRequests.HelperRequestsInput(
    ranges: helperRequests.RowsInput(items: intrlay.SequenceInput.from([
      helperRequests.LanesInput(values: intrlay.FixedArrayInput.from([1, 2])),
    ])),
    components: helperRequests.RowsInput(items: intrlay.SequenceInput.from([
      helperRequests.LanesInput(values: intrlay.FixedArrayInput.from([3, 4])),
    ])),
  ));
  check(helperPair.ranges.items.runtimeType == helperPair.components.items.runtimeType);
  absent(() { (helperPair.ranges.items as dynamic).getValuesComponent(0, 0); });
  helperPair.components.items.setValuesComponent(0, 1, 11);
  check(helperPair.storage.data.getFloat32(helperPair.components.items.getValuesByteOffset(0) + 4, Endian.little) == 11);
  final point = nestedFlatten.PointInput(x: 1, y: 2);
  final pointRow = intrlay.FixedArrayInput.from([point, point]);
  final nestedRanges = nestedFlatten.NestedRangesView.create(nestedFlatten.NestedRangesInput(
    nested: intrlay.FixedArrayInput.from([pointRow, pointRow]),
    records: intrlay.FixedArrayInput.from([nestedFlatten.ItemInput(point: point), nestedFlatten.ItemInput(point: point)]),
    primitives: intrlay.FixedArrayInput.from([intrlay.FixedArrayInput.from([3, 4]), intrlay.FixedArrayInput.from([5, 6])]),
    optional: [point, null],
    numeric: intrlay.FixedArrayInput.from([Float32List.fromList([7, 8]), Float32List.fromList([9, 10])]),
  ));
  check(nestedRanges.nested11YByteOffset == 28 && nestedRanges.records1PointYByteOffset == 44);
  check(nestedRanges.primitives11ByteOffset == 60 && nestedRanges.nested11YByteLength == 4);
  check(nestedRanges.optional1ByteOffset == 76 && nestedRanges.optional1ByteLength == 12);
  check(nestedRanges.numeric1ByteOffset == 96 && nestedRanges.numeric1ByteLength == 8);
  check(nestedRanges.storage.data.getFloat32(nestedRanges.numeric1ByteOffset, Endian.little) == 9);
  check(nestedRanges.storage.data.getFloat32(nestedRanges.records1PointYByteOffset, Endian.little) == 2);
  check(nestedRanges.storage.data.getFloat32(nestedRanges.primitives11ByteOffset, Endian.little) == 6);
  absent(() { (nestedRanges as dynamic).nested11Y; });
  final nestedComponents = nestedFlatten.NestedComponentsView.create(nestedFlatten.NestedComponentsInput(
    items: intrlay.FixedArrayInput.from([
      nestedFlatten.NumericLeafInput(values: intrlay.FixedArrayInput.from([1, 2])),
      nestedFlatten.NumericLeafInput(values: intrlay.FixedArrayInput.from([3, 4])),
    ]),
  ));
  nestedComponents.setItems1ValuesComponent(1, 9);
  check(nestedComponents.getItems1ValuesComponent(1) == 9 && nestedComponents.getItems0ValuesComponent(1) == 2);
  absent(() { (nestedComponents as dynamic).items1Values; });

  final unusedPoint = unusedIndexed.PointInput(x: 1, y: 2);
  final unusedItems = intrlay.SequenceInput.from([unusedIndexed.ItemInput(point: unusedPoint)]);
  final hiddenIndexed = unusedIndexed.UnusedIndexedView.create(unusedIndexed.UnusedIndexedInput(
    items: unusedItems,
    points: intrlay.FixedArrayInput.from([unusedPoint, unusedPoint]),
    ranges: unusedItems,
    fixedRanges: intrlay.FixedArrayInput.from([
      unusedIndexed.ItemInput(point: unusedPoint), unusedIndexed.ItemInput(point: unusedPoint),
    ]),
    wrappedRanges: unusedItems,
  ));
  absent(() { (hiddenIndexed.items as dynamic).getPoint(0); });
  absent(() { (hiddenIndexed.items as dynamic)[0]; });
  absent(() { (hiddenIndexed.points as dynamic).setX(0, 5.0); });
  final inheritedRanges = unusedIndexed.InheritedRangesView.create(unusedIndexed.InheritedRangesInput(items: unusedItems));
  for (final dynamic collection in [hiddenIndexed.ranges, hiddenIndexed.fixedRanges, hiddenIndexed.wrappedRanges, inheritedRanges.items]) {
    absent(() { collection.getPoint(0); });
    absent(() { collection[0]; });
    check(collection.getPointByteLength(0) == 8);
    check(collection.storage.data.getFloat32(collection.getPointByteOffset(0), Endian.little) == 1);
  }
  final readIndexedPoint = unusedIndexed.ReadPointView.create(unusedIndexed.ReadPointInput(point: unusedPoint));
  absent(() { (readIndexedPoint.point as dynamic).x = 5.0; });
  final liveIndexed = usedIndexed.UsedIndexedView.create(usedIndexed.UsedIndexedInput(
    items: intrlay.SequenceInput.from([usedIndexed.ItemInput(point: usedIndexed.PointInput(x: 1, y: 2))]),
  ));
  liveIndexed.items.getPoint(0).x = 8;
  check(liveIndexed.items.getPoint(0).x == 8);

  final lanes = Float32List.fromList([3, 4]);
  final numericValues = numericUses.NumericUsesView.create(numericUses.NumericUsesInput(
    optional: lanes, boxed: lanes, sequence: intrlay.SequenceInput.from([lanes]),
    fixed: intrlay.FixedArrayInput.from([lanes, lanes]), wrapped: lanes,
  ));
  check(numericValues.optional![1] == 4 && numericValues.boxed[1] == 4 && numericValues.sequence[0][1] == 4);
  check(numericValues.fixed[1][1] == 4 && numericValues.wrapped[1] == 4);
  final readNumericPoint = numericUses.ReadPointView.create(numericUses.ReadPointInput(point: numericUses.PointInput(x: 1, y: 2)));
  absent(() { (readNumericPoint.point as dynamic).x = 5.0; });
  final exportedNumeric = numericExport.NumericView.create(lanes);
  exportedNumeric.point.x = 7;
  check(exportedNumeric.point.x == 7);
  const targetSample = projectionTargets.SampleInput(value: 3, valueByteOffset: 321);
  const targetEvent = projectionTargets.EventValueInput(sample: targetSample);
  const targetPoint = projectionTargets.PointInput(x: 1, y: 2);
  final ordinaryTargets = projectionTargets.OrdinaryView.create(const projectionTargets.OrdinaryInput(
    sample: targetSample, event: targetEvent, point: targetPoint,
  ));
  check(ordinaryTargets.sample.valueByteOffset == 321);
  check(ordinaryTargets.event.valueSampleValueByteOffset == 321);
  absent(() { (ordinaryTargets.sample as dynamic).valueByteLength; });
  absent(() { (ordinaryTargets.event as dynamic).valueSampleValueByteLength; });
  absent(() { (ordinaryTargets.point as dynamic).x = 5.0; });
  final targetRow = intrlay.FixedArrayInput.from([targetSample, targetSample]);
  final projectedTargets = projectionTargets.ProjectedView.create(projectionTargets.ProjectedInput(
    flat: targetSample, nested: intrlay.FixedArrayInput.from([targetRow, targetRow]),
    indexed: intrlay.SequenceInput.from([targetSample]),
    events: [const projectionTargets.EventEmptyInput(), targetEvent], flatPoint: targetPoint,
    indexedPoints: intrlay.SequenceInput.from([targetPoint]),
    inlineOnly: const projectionTargets.InlineOnlyInput(value: 4),
  ));
  check(projectedTargets.flatValueByteOffset == 0 && projectedTargets.flatValueByteOffsetByteOffset == 4);
  check(projectedTargets.nested11ValueByteOffset == 32);
  check(projectedTargets.storage.data.getFloat32(projectedTargets.nested11ValueByteOffset, Endian.little) == 3);
  check(projectedTargets.storage.data.getUint32(projectedTargets.indexed.getValueByteOffsetByteOffset(0), Endian.little) == 321);
  check(projectedTargets.events.getTag(0) == 0);
  check(projectedTargets.events.getTag(1) == 1);
  check(projectedTargets.storage.data.getFloat32(projectedTargets.events.getValueSampleValueByteOffset(1), Endian.little) == 3);
  projectedTargets.flatPointX = 9;
  projectedTargets.indexedPoints.setX(0, 7);
  check(projectedTargets.flatPointX == 9 && projectedTargets.indexedPoints.getX(0) == 7);
  projectedTargets.inlineOnlyValue = 5;
  check(projectedTargets.inlineOnlyValue == 5);
  final exportedPoint = projectionExports.PointView.create(const projectionExports.PointInput(x: 1, y: 2));
  exportedPoint.x = 8;
  check(exportedPoint.x == 8);
  print('child view configuration: Dart passed');
}
