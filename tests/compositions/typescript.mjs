import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { zeroed } from "../../packages/typescript/dist/index.js";
import { ZeroedArraysView } from "../../target/compositions/bindings.js";
import { runInNewContext } from "node:vm";
import { NumericItemsView } from "../../target/compositions/bindings.js";
import { EncodingBuffer } from "../../packages/typescript/dist/index.js";
import { RenamedListView, ProjectionSetView, WrappedScalarsView, CachedChildrenView, OptionalRootView, CachedRootView } from "../../target/compositions/bindings.js";
import { ContainersView, TupleRootView, TupleTreeView } from "../../target/compositions/bindings.js";
import { NumericOwnersView } from "../../target/compositions/bindings.js";

const numericOwnerBytes = new Uint8Array(readFileSync("target/compositions/numeric-owners.bin"));
const numericOwnerInput = {
    boxed: [11, 12], vector: [21, 22], rc: [31, 32], arc: [41, 42],
    optional: [51, 52], empty: [], absent: null, nested: [[61, 62], []],
    fixedSlices: [[71, 72], []], fixed: [81, 82], matrix: [[1, 2], [3, 4]],
    tuple: ["tuple", [91, 92], 93], flattened: [101, [102, 103]],
    optionalItems: [111, null], structural: [121, 122], rcText: "rc", arcText: "arc", rcVector: [131, 132],
};
assert.deepEqual(NumericOwnersView.create(numericOwnerInput).storage.bytes, numericOwnerBytes);
const numericOwnerBacking = new Uint8Array(numericOwnerBytes.length + 32).fill(0xa5);
const owners = NumericOwnersView.createInPlace(numericOwnerBacking.subarray(16), numericOwnerInput);
for (const [name, first] of [["boxed", 11], ["vector", 21], ["rc", 31], ["arc", 41]]) {
    const values = owners[name];
    assert.ok(values instanceof Uint32Array);
    assert.equal(values, owners[name]);
    assert.equal(values[0], first);
    assert.equal(owners[`${name}ByteOffset`], values.byteOffset - owners.storage.bytes.byteOffset);
    assert.equal(owners[`${name}ByteLength`], values.byteLength);
    values[0] += 1;
    assert.equal(owners.storage.u32[owners[`${name}ByteOffset`] >>> 2], first + 1);
}
assert.ok(owners.empty instanceof Uint32Array && owners.empty.length === 0);
assert.equal(owners.emptyByteLength, 0);
assert.equal(owners.optional[1], 52);
assert.equal(owners.absent, null);
assert.equal(owners.nested.get(0)[1], 62);
assert.equal(owners.nested.get(1).length, 0);
assert.equal(owners.fixedSlices.get(0)[1], 72);
assert.equal(owners.fixed[1], 82);
assert.equal(owners.matrix.length, 2);
assert.deepEqual([...owners.matrix.get(1)], [3, 4]);
assert.equal(owners.tuple.field0, "tuple");
assert.equal(owners.tuple.field1[1], 92);
assert.equal(owners.flattenedField11, 103);
assert.equal(owners.optionalItems.get(0), 111);
assert.equal(owners.optionalItems.get(1), null);
assert.equal(owners.structural.get(1), 122);
assert.equal(owners.rcText, "rc");
assert.equal(owners.arcText, "arc");
assert.equal(owners.rcVector[1], 132);
assert.ok(numericOwnerBacking.subarray(0, 16).every(value => value === 0xa5));
assert.ok(numericOwnerBacking.subarray(16 + numericOwnerBytes.length).every(value => value === 0xa5));

const containers = ContainersView.create({
    label: "containers 🦀", items: [[11, "first"], [22, "second"]], pair: [7, 91n],
    nested: [[53], ["nested", true]], wide: [1, 2, 3, 4, 5, 6, 7, 8, 9],
    first: 42, second: 42, samples: new Uint32Array([3, 5, 8]), note: "shared",
});
assert.deepEqual(containers.storage.bytes, new Uint8Array(readFileSync("target/compositions/containers.bin")));
assert.equal(containers.items.get(1).field1, "second");
assert.equal(containers.nested.field0.field0, 53);
assert.equal(containers.wide.field8, 9);
containers.pair.field0 = 8;
assert.equal(containers.pair.field0, 8);
assert.equal(TupleRootView.create([17, "root"]).field1, "root");
const tupleTree = TupleTreeView.create({ value: [1], children: [[{ value: [2], children: [[]] }]] });
assert.equal(tupleTree.children.field0.get(0).value.field0, 2);

const zeroInput = { id: 73, points: zeroed(3), flags: zeroed(4), matrix: zeroed(), present: zeroed(), nested: [zeroed(2), zeroed(0)], empty: zeroed() };
const zeroExpected = new Uint8Array(readFileSync("target/compositions/zeroed.bin"));
const zeroOwner = new Uint8Array(zeroExpected.length + 32).fill(0xa5);
const zeroView = ZeroedArraysView.createInPlace(zeroOwner.subarray(16), zeroInput);
assert.deepEqual(zeroView.storage.bytes, zeroExpected);
assert.deepEqual(ZeroedArraysView.create(zeroInput).storage.bytes, zeroExpected);
assert.ok(zeroOwner.subarray(0, 16).every(value => value === 0xa5));
assert.ok(zeroOwner.subarray(16 + zeroExpected.length).every(value => value === 0xa5));
zeroView.matrix.fill(7);
zeroView.setMatrix(zeroed());
assert.deepEqual(zeroView.storage.bytes, zeroExpected);

const renamed = RenamedListView.create({ items: [{ first: 11, second: 22 }] });
assert.equal(renamed.items.getFirst(0), 11);
renamed.items.setSecond(0, 23);
assert.equal(renamed.items.get(0).second, 23);

const padded = value => new Float32Array([value, 0, 0, 0]);
const projections = ProjectionSetView.create({ direct: padded(1), optional: padded(2), boxed: padded(3), elements: [padded(4), padded(5)], readOnly: padded(6) });
for (const value of [projections.direct, projections.optional, projections.boxed, projections.elements.get(0), projections.readOnly]) {
    assert.ok(value instanceof Float32Array);
    assert.equal(value.length, 4);
}
assert.equal(projections.direct, projections.direct);
assert.equal(projections.optional, projections.optional);
assert.equal(projections.boxed, projections.boxed);
projections.optional[0] = 7;
assert.equal(projections.optional[0], 7);

const wrapped = WrappedScalarsView.create({ present: 1, absent: null, boxed: 2, nested: { value: 3 }, missingInner: { value: null }, items: [4, null], fixed: [5, null], readOnly: 6, writeOnly: 7 });
const length = wrapped.storage.bytes.byteLength;
wrapped.present = 11;
wrapped.absent = 12;
wrapped.setBoxed(13);
wrapped.setNested(14);
wrapped.setMissingInner(15);
wrapped.items.set(0, 16);
wrapped.items.set(1, 17);
wrapped.fixed.set(0, 18);
wrapped.fixed.set(1, 19);
wrapped.setWriteOnly(20);
assert.deepEqual([wrapped.present, wrapped.absent, wrapped.boxed, wrapped.nested.value, wrapped.missingInner.value], [11, 12, 13, 14, 15]);
assert.deepEqual([wrapped.items.get(0), wrapped.items.get(1), wrapped.fixed.get(0), wrapped.fixed.get(1)], [16, null, 18, 19]);
assert.equal(wrapped.storage.bytes.byteLength, length);
assert.equal("setReadOnly" in wrapped, false);
assert.equal("writeOnly" in wrapped, false);

for (const value of [null, { value: null }, { value: 2 }]) {
    const root = OptionalRootView.create(value);
    root.set(9);
    assert.deepEqual(root.value, value === null ? null : { value: 9 });
}

const cached = CachedChildrenView.create({ child: { value: 1 }, absent: null, boxed: { value: 2 }, nested: { value: { value: 3 } }, choice: { kind: 1 /* CachedChoiceKind.Value */, child: { value: 4 } } });
assert.equal(cached.child, cached.child);
assert.equal(cached.absent, null);
assert.equal(cached.boxed, cached.boxed);
assert.equal(cached.nested, cached.nested);
assert.equal(cached.choice.valueChild, cached.choice.valueChild);
cached.child.value = 21;
cached.nested.value.value = 22;
cached.choice.valueChild.value = 23;
assert.equal(cached.child.value, 21);
assert.equal(cached.nested.value.value, 22);
assert.equal(cached.choice.valueChild.value, 23);
const cachedRoot = CachedRootView.create({ value: 1 });
assert.equal(cachedRoot.value, cachedRoot.value);
cachedRoot.value.value = 2;
assert.equal(cachedRoot.value.value, 2);

const numeric = NumericItemsView.create({ items: [{ id: 7, position: [1, 2, 3], matrix: [[4, 5], [6, 7]], padded: padded(8), weights: new Float32Array([9, 10]), wide: [11n, 12n], readOnly: [13, 14], writeOnly: [15, 16] }] });
const owner = new Uint8Array(numeric.storage.bytes.length + 32);
owner.set(numeric.storage.bytes, 16);
const items = new NumericItemsView(owner.subarray(16)).items;
assert.equal(items.getPositionComponent(0, 2), 3);
assert.equal(items.getMatrixComponent(0, 3), 7);
assert.equal(items.getPaddedComponent(0, 0), 8);
assert.equal(items.getPaddedComponent(0, 3), 0);
assert.equal(items.getWeightsComponent(0, 1), 10);
assert.equal(items.getWideComponent(0, 1), 12n);
items.setPositionComponent(0, 1, 22);
items.setWeightsComponent(0, 0, 29);
items.setWideComponent(0, 1, 0xffffffffffffffffn);
items.setWriteOnlyComponent(0, 0, -123);
assert.equal(items.get(0).position[1], 22);
assert.equal(items.get(0).weights[0], 29);
assert.equal(items.get(0).wide[1], 0xffffffffffffffffn);
assert.equal(items.get(0).writeOnly[0], -123);
items.get(0).position[2] = 33;
assert.equal(items.getPositionComponent(0, 2), 33);
assert.equal('setReadOnlyComponent' in items, false);
assert.equal('getWriteOnlyComponent' in items, false);

const input = { items: [{ id: 7, position: [1, 2, 3], matrix: [[4, 5], [6, 7]], padded: padded(8), weights: new Float32Array([9, 10]), wide: [11n, 12n], readOnly: [13, 14], writeOnly: [15, 16] }] };
const reusableBytes = new Uint8Array(numeric.storage.bytes.length + 64).fill(0xcc);
const encoding = new EncodingBuffer(reusableBytes.subarray(16));
const first = NumericItemsView.createInPlace(encoding, input);
const firstStorage = first.storage;
const firstF32 = firstStorage.f32;
const second = NumericItemsView.createInPlace(encoding, input);
assert.equal(second.storage, firstStorage);
assert.equal(second.storage.f32, firstF32);
assert.deepEqual(second.storage.bytes, NumericItemsView.create(input).storage.bytes);
input.items[0].weights = [];
const resized = NumericItemsView.createInPlace(encoding, input);
assert.notEqual(resized.storage, firstStorage);
assert.deepEqual(resized.storage.bytes, NumericItemsView.create(input).storage.bytes);
assert.equal(reusableBytes[15], 0xcc);
assert.equal(reusableBytes[16 + numeric.storage.bytes.length], 0xcc);
assert.throws(() => NumericItemsView.createInPlace(new EncodingBuffer(new Uint8Array(8)), input));
assert.throws(() => NumericItemsView.createInPlace(new EncodingBuffer(reusableBytes.subarray(1)), input));
const realmBytes = runInNewContext("new Uint8Array(byteLength)", { byteLength: numeric.storage.bytes.length });
const realmResult = NumericItemsView.createInPlace(realmBytes, input);
assert.deepEqual([...realmResult.storage.bytes], [...NumericItemsView.create(input).storage.bytes]);
console.log("TypeScript composition regressions passed");
