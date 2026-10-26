import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
const pkg = resolve(process.argv[2]);
const { default: init, Core, WebCounter } = await import(pathToFileURL(resolve(pkg, 'simple_ffi_web.js')));
await init({ module_or_path: await readFile(resolve(pkg, 'simple_ffi_web_bg.wasm')) });
const core = new Core();
const initialCounter = new WebCounter(12);
assert.equal(initialCounter.getCount(), 12);
initialCounter.free();
const profile = {
  displayName: 'Samson', scores: [1, 2, -3], note: null,
  flags: new Map([['local', true], ['remote', false]]),
  bigCount: 18446744073709551615n,
  choice: {tag: 'Named', value: {displayLabel: 'Hello', weight: 7}},
  nested: [null, {tag: 'Empty'}, {tag: 'Named', value: {displayLabel: 'World', weight: -4}}],
};
assert.deepEqual(core.roundTripProfile(profile), profile);
assert.deepEqual(core.optionalProfile(profile), profile);
assert.equal(core.optionalProfile(null), null);
assert.deepEqual(core.roundTripChoices(profile.nested.filter(v => v !== null)), profile.nested.filter(v => v !== null));
assert.deepEqual(core.validateProfile(profile), profile);
assert.throws(() => core.validateProfile({...profile, displayName: ''}), error => {
  assert.ok(error instanceof Error);
  assert.deepEqual(error.details, {tag: 'InvalidInput', value: {reason: 'Name is required'}});
  return true;
});
assert.throws(() => core.roundTripProfile({...profile, scores: ['wrong type']}), TypeError);
assert.deepEqual(await core.asyncProfile(profile), profile);
await assert.rejects(core.asyncProfile({...profile, displayName:''}), error => {
  assert.deepEqual(error.details, {tag:'InvalidInput', value:{reason:'Name is required'}});
  return true;
});
assert.equal(core.getCount(), 0);
assert.equal(core.increment(), 1);
assert.equal(core.getCount(), 1);
const counter = core.makeWebCounter();
assert.ok(counter instanceof WebCounter);
assert.equal(counter.increment(), 1);
const sameCounter = core.roundTripObject(counter);
assert.ok(sameCounter instanceof WebCounter);
assert.equal(sameCounter.increment(), 2);
assert.equal(counter.getCount(), 2);
const envelope = core.roundTripObjectEnvelope({counter, label:'nested object'});
assert.ok(envelope.counter instanceof WebCounter);
assert.equal(envelope.counter.increment(), 3);
assert.equal(counter.getCount(), 3);
assert.throws(() => core.roundTripObject({}), TypeError);
sameCounter.free();
envelope.counter.free();
counter.free();
assert.throws(() => core.roundTripObject(counter), TypeError);
core.free();
console.log('PASS: object references/lifetime, parameterized constructor, records, enum payloads, nested vectors/options, Map, full-range u64 BigInt, Result success/error details, invalid inputs, async Promise, greeting/counter');
