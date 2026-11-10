import { AsyncFactory, FactoryOnly, roundTripTime, roundTripBytes, macroRoundTrip, externalProfile, hiddenRoundTrip, globRoundTrip, type JffiTimestamp, type JffiDuration } from './pkg/simple_ffi_web.js';
const timestamp: JffiTimestamp = 1234567890123456789n;
const duration: JffiDuration = 999999999999999999n;
roundTripTime({timestamp,duration});
const bytes: Uint8Array = roundTripBytes(new Uint8Array([0,255]));
const factory: Promise<AsyncFactory> = AsyncFactory.new(1);
const alternate: FactoryOnly = FactoryOnly.create(1);
macroRoundTrip({value:'generated'});
externalProfile({id:18446744073709551615n,label:'external'});
hiddenRoundTrip({message:'alias'});globRoundTrip({value:true});
// @ts-expect-error preserving nanoseconds requires bigint
roundTripTime({timestamp:Date.now(),duration:1n});
// @ts-expect-error bytes are typed buffers
roundTripBytes('bad bytes');
// @ts-expect-error async constructors return a Promise
const synchronous:AsyncFactory=AsyncFactory.new(1);
// @ts-expect-error external custom integers preserve bigint
externalProfile({id:42,label:'bad integer'});
