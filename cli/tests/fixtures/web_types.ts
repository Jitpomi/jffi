import { Core, type WebProfile, type WebChoice, type WebFailure } from './pkg/simple_ffi_web.js';
const core = new Core();
const choice: WebChoice = {tag:'Named', value:{displayLabel:'test', weight:1}};
const profile: WebProfile = {displayName:'test', scores:[1], note:null, flags:new Map(), bigCount:1n, choice, nested:[null, choice]};
const roundTrip: WebProfile = core.roundTripProfile(profile);
const optional: WebProfile | null = core.optionalProfile(null);
const choices: WebChoice[] = core.roundTripChoices([choice]);
const asyncResult: Promise<WebProfile> = core.asyncProfile(profile);
const error: WebFailure = {tag:'InvalidInput', value:{reason:'missing'}};
// @ts-expect-error Records must preserve field types.
core.roundTripProfile({...profile, scores:['wrong']});
// @ts-expect-error u64 is bigint, not a lossy number.
core.roundTripProfile({...profile, bigCount:1});
// @ts-expect-error Tagged enums must constrain their payload.
const invalidChoice: WebChoice = {tag:'Named', value:{displayLabel:4, weight:1}};
// @ts-expect-error Async exports must return a Promise.
const invalidAsync: WebProfile = core.asyncProfile(profile);
void [roundTrip, optional, choices, asyncResult, error, invalidChoice, invalidAsync];
import { WebCounter, type WebObjectEnvelope } from './pkg/simple_ffi_web.js';
const counter = new WebCounter(12);
const nestedObject: WebObjectEnvelope = core.roundTripObjectEnvelope({counter, label:'typed handle'});
const returnedCounter: WebCounter = core.makeWebCounter();
// @ts-expect-error An exported object argument must be an actual typed object.
core.roundTripObject({});
// @ts-expect-error Constructor arguments must retain Rust numeric types.
new WebCounter('bad');
void [nestedObject, returnedCounter];
