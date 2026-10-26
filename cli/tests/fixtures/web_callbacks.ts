import {Core,type WebFormatter,type WebService,type WebProfile,type WebCustomRecord,type WebUserId,type WebEmail,type WebIp,type WebLabel,type JffiRustError,type WebFailure} from './pkg/simple_ffi_web.js';
const core=new Core();
const userId:WebUserId=18446744073709551615n;
const email:WebEmail='sam@example.com';
const address:WebIp='127.0.0.1';
const label:WebLabel='example';
const value:WebCustomRecord={userId,email,address};
const profile:WebProfile={displayName:'Samson',scores:[1],note:null,flags:new Map(),bigCount:12n,choice:{tag:'Empty'},nested:[null]};
const formatter:WebFormatter={onCount(count){return count+1;},formatProfile(input,email){return {...input,displayName:email};}};
const service:WebService={consume(input){return input;},transform(input){return input;},async transformAsync(input){return input;}};
core.retainFormatter(formatter);
const formatted:WebProfile=core.useRetainedFormatter(profile,email);
const syncResult:WebCustomRecord=core.invokeWebService(service,value);
const asyncResult:Promise<WebCustomRecord>=core.invokeWebServiceAsync(service,value);
const returnedService:WebService=core.makeWebService();
const labels:WebLabel[]=core.roundTripLabels([label]);
const error:JffiRustError<WebFailure>=Object.assign(new Error('example'),{details:{tag:'InvalidInput' as const,value:{reason:'example'}}});
// @ts-expect-error Callback interfaces must include every exported method.
core.retainFormatter({onCount(count:number){}});
// @ts-expect-error Synchronous callbacks must not return a Promise for a record.
core.retainFormatter({onCount(){},async formatProfile(){return profile;}});
// @ts-expect-error Callback argument types must match their exported Rust types.
core.retainFormatter({onCount(count:string){},formatProfile(value:WebProfile){return value;}});
// @ts-expect-error Async callback results must match the declared record.
core.invokeWebServiceAsync({consume(input:WebCustomRecord){return input;},transform(input:WebCustomRecord){return input;},async transformAsync(){return 'invalid';}},value);
// @ts-expect-error Custom u64 representations preserve bigint, not number.
core.roundTripCustom({...value,userId:12});
// @ts-expect-error Custom String-backed types preserve string representation.
core.roundTripCustom({...value,email:42});
void [formatted,syncResult,asyncResult,returnedService,labels,error];
import type {WebWide} from './pkg/simple_ffi_web.js';
const wide:WebWide=core.makeWide();
const wideResult:number=wide.sum(1,2,3,4,5,6,7,8,9);
const widePromise:Promise<number>=wide.sumAsync(1,2,3,4,5,6,7,8,9);
const ownedResult:number=core.ownedCount();
// @ts-expect-error Wide callback signatures retain all required arguments.
wide.sum(1);
void [wideResult,widePromise,ownedResult];
