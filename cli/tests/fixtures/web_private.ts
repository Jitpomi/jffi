import {PrivateCounter,privateRoundTrip,privateCallback,privateCancelCallback,privateId,generatedPrivateRoundTrip,type PrivateProfile,type PrivateId,type GeneratedPrivateRecord} from './pkg/simple_ffi_web.js';
const profile:PrivateProfile={label:'private',count:42};
const returned:PrivateProfile=privateRoundTrip(profile);
const counter=new PrivateCounter(returned.count);
const value:Promise<number>=counter.asyncValue();
const id:PrivateId=privateId(18446744073709551615n);
const callback:number=privateCallback({transform(value){return value+1;}});
const cancelled:Promise<boolean>=privateCancelCallback({async wait(){return 42;}});
const macro:GeneratedPrivateRecord=generatedPrivateRoundTrip({first:'one',second:'two'});
// @ts-expect-error private Rust fields still have precise exported types
privateRoundTrip({label:'private',count:'wrong'});
// @ts-expect-error legacy macro fields are discovered from metadata
 generatedPrivateRoundTrip({first:'one'});
// @ts-expect-error callback signatures remain precise
privateCallback({transform(){return 'wrong';}});
// @ts-expect-error custom u64 remains bigint
privateId(42);
void [value,id,callback,cancelled,macro];counter.free();
