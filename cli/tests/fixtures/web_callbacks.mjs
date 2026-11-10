import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {pathToFileURL} from 'node:url';
import {resolve} from 'node:path';
const pkg=resolve(process.argv[2]);
const {default:init,Core}=await import(pathToFileURL(resolve(pkg,'simple_ffi_web.js')));
await init({module_or_path:await readFile(resolve(pkg,'simple_ffi_web_bg.wasm'))});
const core=new Core();
const custom={userId:18446744073709551615n,email:'  SAM@EXAMPLE.COM  ',address:'127.0.0.1'};
const normalized={...custom,email:'sam@example.com'};
assert.deepEqual(core.roundTripCustom(custom),normalized);
assert.deepEqual(core.roundTripCustom(custom),normalized);
assert.throws(()=>core.roundTripCustom({...custom,email:'invalid'}),error=>error instanceof TypeError && /Email must contain @/.test(error.message));
assert.throws(()=>core.roundTripCustom({...custom,address:'invalid'}),TypeError);
assert.deepEqual(core.roundTripEmails([null,' SAM@EXAMPLE.COM ',null]),[null,'sam@example.com',null]);
assert.deepEqual(core.roundTripLabels([' a ','b']),['a','b']);
assert.throws(()=>core.roundTripLabels([' ']),error=>error instanceof TypeError && /Label cannot be empty/.test(error.message));
const profile={displayName:'Samson',scores:[1,2],note:null,flags:new Map([['local',true]]),bigCount:12n,choice:{tag:'Empty'},nested:[null]};
const counts=[];
// A JavaScript void callback is allowed to return an ignored value.
let formatter={
  marker:'bound receiver',
  onCount(count){assert.equal(this.marker,'bound receiver');assert.equal(core.greeting(),'Hello from JFFI');return counts.push(count);},
  formatProfile(value,email){assert.equal(email,'sam@example.com');return {...value,displayName:`${value.displayName} via callback`};},
};
core.retainFormatter(formatter);
formatter=null;
assert.equal(core.useRetainedFormatter(profile,custom.email).displayName,'Samson via callback');
assert.deepEqual(counts,[0]);
assert.throws(()=>core.retainFormatter({onCount(){}}),TypeError);
assert.equal(core.useRetainedFormatter(profile,custom.email).displayName,'Samson via callback');
const details={tag:'InvalidInput',value:{reason:'Declared JavaScript callback failure'}};
const declared=()=>Object.assign(new Error('declared error'),{details});
core.retainFormatter({onCount(){},formatProfile(){throw declared();}});
assert.throws(()=>core.useRetainedFormatter(profile,custom.email),error=>{assert.deepEqual(error.details,details);return true;});
core.retainFormatter({onCount(){},formatProfile(){throw new Error('unexpected JavaScript callback failure');}});
assert.throws(()=>core.useRetainedFormatter(profile,custom.email),error=>{
  assert.equal(error.details.tag,'InvalidInput');
  assert.match(error.details.value.reason,/Unexpected callback failure/);
  assert.match(error.details.value.reason,/unexpected JavaScript callback failure/);
  return true;
});
core.retainFormatter({onCount(){},formatProfile(){return {...profile,scores:['invalid']};}});
assert.throws(()=>core.useRetainedFormatter(profile,custom.email),error=>{assert.match(error.details.value.reason,/Unexpected callback failure/);return true;});
core.clearFormatter();
assert.throws(()=>core.useRetainedFormatter(profile,custom.email),error=>{assert.equal(error.details.value.reason,'No formatter');return true;});
const rustService=core.makeWebService();
assert.deepEqual(rustService.transform(custom),normalized);
assert.deepEqual(await rustService.transformAsync(custom),normalized);
assert.throws(()=>rustService.transform({...custom,email:'fail@example.com'}),error=>{assert.equal(error.details.value.reason,'Rust service rejected email');return true;});
await assert.rejects(rustService.transformAsync({...custom,email:'fail@example.com'}),error=>{assert.equal(error.details.value.reason,'Rust service rejected email');return true;});
let service={
  label:'original JS service',
  consume(value){return this.transform(value);},transform(value){assert.equal(this.label,'original JS service');return {...value,email:'changed@example.com'};},
  async transformAsync(value){await Promise.resolve();assert.equal(this.label,'original JS service');return {...value,email:'async@example.com'};},
};
assert.deepEqual(core.invokeWebService(service,custom),{...normalized,email:'changed@example.com'});
assert.deepEqual(await core.invokeWebServiceAsync(service,custom),{...normalized,email:'async@example.com'});
assert.deepEqual(core.invokeOwnedService(service,custom),{...normalized,email:'changed@example.com'});
assert.deepEqual(rustService.consume(custom),normalized);
assert.equal(core.ownedCount(),0);
const roundTripped=core.roundTripService(service);
service=null;
assert.deepEqual(roundTripped.transform(custom),{...normalized,email:'changed@example.com'});
assert.deepEqual(await roundTripped.transformAsync(custom),{...normalized,email:'async@example.com'});
const record=core.roundTripServiceRecord({service:roundTripped,label:'nested callback'});
assert.equal(record.label,'nested callback');
assert.deepEqual(await record.service.transformAsync(custom),{...normalized,email:'async@example.com'});
await assert.rejects(core.invokeWebServiceAsync({consume(value){return this.transform(value);},transform(value){return value;},async transformAsync(){throw declared();}},custom),error=>{assert.deepEqual(error.details,details);return true;});
await assert.rejects(core.invokeWebServiceAsync({consume(value){return this.transform(value);},transform(value){return value;},async transformAsync(){throw new Error('unexpected async failure');}},custom),error=>{assert.match(error.details.value.reason,/Unexpected callback failure/);return true;});
let recursive;
const recursiveService={
  consume(value){return this.transform(value);},transform(value){return value.userId>0n?recursive.transform({...value,userId:value.userId-1n}):value;},
  async transformAsync(value){return this.transform(value);},
};
recursive=core.roundTripService(recursiveService);
assert.equal(recursive.transform({...custom,userId:3n}).userId,0n);
const wide={sum(...args){return args.reduce((a,b)=>a+b,0);},async sumAsync(...args){await Promise.resolve();return this.sum(...args);}};
assert.equal(core.invokeWide(wide),45);
assert.equal(await core.invokeWideAsync(wide),45);
const nativeWide=core.makeWide();
assert.equal(nativeWide.sum(1,2,3,4,5,6,7,8,9),45);
assert.equal(await nativeWide.sumAsync(1,2,3,4,5,6,7,8,9),45);
assert.equal(core.increment(),1);
assert.equal(core.getCount(),1);
core.free();
console.log('PASS: custom/newtype/remote converters, normalization, invalid values, nested custom values, Box/Arc callback traits, retained and returned callbacks, receiver binding/re-entry, Arc<Self> ownership, wide-arity callbacks, sync/async methods, declared/unexpected errors, nested callback records');
