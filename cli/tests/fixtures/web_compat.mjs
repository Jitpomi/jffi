import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {pathToFileURL} from 'node:url';
import {resolve} from 'node:path';
const pkg=resolve(process.argv[2]);
const api=await import(pathToFileURL(resolve(pkg,'simple_ffi_web.js')));
await api.default({module_or_path:await readFile(resolve(pkg,'simple_ffi_web_bg.wasm'))});
for (const timestamp of [0n,1n,9007199254987654321n]) {
    const value={timestamp,duration:18446744073709551615999999999n};
    assert.deepEqual(api.roundTripTime(value),value);
}
assert.throws(()=>api.roundTripTime({timestamp:-1n,duration:0n}),/SystemTime range/);
assert.throws(()=>api.roundTripTime({timestamp:0n,duration:-1n}));
assert.throws(()=>api.roundTripTime({timestamp:0n,duration:18446744073709551616000000000n}));
assert.deepEqual(api.roundTripBytes(new Uint8Array([0,1,127,128,255])),new Uint8Array([0,1,127,128,255]));
assert.deepEqual(api.externalProfile({id:18446744073709551615n,label:"external"}),{id:18446744073709551615n,label:"external"});
assert.deepEqual(api.hiddenRoundTrip({message:"alias"}),{message:"alias"});
assert.deepEqual(api.globRoundTrip({value:true}),{value:true});
assert.equal(api.buildGeneratedValue(),91);
assert.deepEqual(api.namedMacroRoundTrip({label:"parameterized macro"}),{label:"parameterized macro"});
const renamedRecord=api.publicRecord??api.rustRecord;assert.deepEqual(renamedRecord({label:"renamed"}),{label:"renamed"});
const RenamedObject=api.PublicObject??api.RustNamed;const renamedObject=new RenamedObject(17);assert.equal((renamedObject.readValue??renamedObject.actualValue).call(renamedObject),17);renamedObject.free();
const factory=await api.AsyncFactory.new(19);
assert.equal(factory.value(),19);factory.free();
const alternate=api.FactoryOnly.create(31);assert.equal(alternate.value(),31);alternate.free();
const core=new api.Core();
const result={values:api.compatibilityValues(),macro:api.macroRoundTrip({value:'macro'}).value,included:api.includedValue(),factory:31,greeting:core.greeting(),count:[core.getCount(),core.increment(),core.getCount()],custom:api.standaloneCustom(' PERSON@Example.COM ')};
const choice={tag:'Named',value:{displayLabel:'payload',weight:-7}};
const profile={displayName:'Samson',scores:[-1,0,99],note:null,flags:new Map([['local',true]]),bigCount:18446744073709551615n,choice,nested:[null,{tag:'Empty'},choice]};
const returned=api.standaloneProfile(profile);
result.record={name:returned.displayName,scores:returned.scores,note:returned.note,flags:Object.fromEntries(returned.flags),bigCount:String(returned.bigCount),choice:returned.choice,nested:returned.nested};
const events=[];
core.retainFormatter({onCount(count){events.push(count);},formatProfile(value,email){return {...value,displayName:value.displayName+' / '+email};}});
result.callback={name:core.useRetainedFormatter(profile,' PERSON@Example.COM ').displayName,events};core.clearFormatter();
try{api.standaloneFailure();}catch(error){result.declaredError=error.details.value.reason;}
result.async=await api.standaloneAsync('async native/web');
result.bytes=Array.from(api.roundTripBytes(new Uint8Array([0,1,127,128,255])),b=>b.toString(16).padStart(2,'0')).join('');
const time=api.roundTripTime({timestamp:1600000000123456000n,duration:123456000n});result.time={timestamp:String(time.timestamp),duration:String(time.duration)};
core.free();
const types=await readFile(resolve(pkg,'simple_ffi_web.d.ts'),'utf8');
if (process.env.JFFI_COMPAT_UNIFFI_VERSION?.startsWith("0.31.")) {assert.equal(typeof api.PublicObject,"function");assert.equal(api.RustNamed,undefined);assert.equal(typeof api.publicRecord,"function");assert.match(types,/export type PublicRecord/);}
assert.match(types,/static new\(value: number\): Promise<AsyncFactory>/);
assert.match(types,/timestamp: JffiTimestamp/);assert.match(types,/duration: JffiDuration/);
console.log(JSON.stringify(result));
