import json
import asyncio
from datetime import datetime,timedelta,timezone
from native_compatibility import simple_core as api
assert api.private_live_count()==0
private=api.PrivateCounter(41)
assert private.increment()==42
assert asyncio.run(private.async_value())==42
assert api.private_counter_value(private,[])==42
record=api.private_round_trip(api.PrivateProfile(label="private fields",count=42))
assert (record.label,record.count)==("private fields",42)
class PrivateListener:
    def transform(self,value):return value+1
assert api.private_callback(PrivateListener())==43
assert api.private_id(18446744073709551615)==18446744073709551615
record=api.generated_private_round_trip(api.GeneratedPrivateRecord(first="macro one",second="macro two"))
assert (record.first,record.second)==("macro one","macro two")
del private
assert api.private_live_count()==0
core=api.Core()
result={"values":api.compatibility_values(),"macro":api.macro_round_trip(api.MacroRecord(value="macro")).value,"included":api.included_value(),"factory":api.FactoryOnly.create(31).value(),"greeting":core.greeting(),"count":[core.get_count(),core.increment(),core.get_count()],"custom":api.standalone_custom(" PERSON@Example.COM ")}
choice=api.WebChoice.NAMED(display_label="payload",weight=-7)
profile=api.WebProfile(display_name="Samson",scores=[-1,0,99],note=None,flags={"local":True},big_count=18446744073709551615,choice=choice,nested=[None,api.WebChoice.EMPTY(),choice])
returned=api.standalone_profile(profile)
def normalize_choice(choice):
    if choice is None:return None
    if choice.is_EMPTY():return {"tag":"Empty"}
    return {"tag":"Named","value":{"displayLabel":choice.display_label,"weight":choice.weight}}
result["record"]={"name":returned.display_name,"scores":returned.scores,"note":returned.note,"flags":returned.flags,"bigCount":str(returned.big_count),"choice":normalize_choice(returned.choice),"nested":[normalize_choice(v) for v in returned.nested]}
events=[]
class Formatter:
    def on_count(self,count):events.append(count)
    def format_profile(self,profile,email):
        profile.display_name += " / " + email
        return profile
core.retain_formatter(Formatter())
result["callback"]={"name":core.use_retained_formatter(profile," PERSON@Example.COM ").display_name,"events":events}
core.clear_formatter()
try:api.standalone_failure()
except api.WebFailure.InvalidInput as error:result["declaredError"]=error.reason
result["async"]=asyncio.run(api.standalone_async("async native/web"))
result["bytes"]=api.round_trip_bytes(bytes([0,1,127,128,255])).hex()
value=api.round_trip_time(api.TimeValues(timestamp=datetime(2020,9,13,12,26,40,123456,tzinfo=timezone.utc),duration=timedelta(microseconds=123456)))
delta=value.timestamp-datetime(1970,1,1,tzinfo=timezone.utc)
result["time"]={"timestamp":str(((delta.days*86400+delta.seconds)*1000000+delta.microseconds)*1000),"duration":str(((value.duration.days*86400+value.duration.seconds)*1000000+value.duration.microseconds)*1000)}
print(json.dumps(result,ensure_ascii=False))
