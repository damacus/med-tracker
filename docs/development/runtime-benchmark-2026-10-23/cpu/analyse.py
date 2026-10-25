import datetime
import json
import statistics
import sys
from pathlib import Path

root = Path('/tmp/medtracker-cpu-benchmark')
cases = [('rails','async'),('rails','inline'),('rails','fork'),('loco','async'),('loco','inline')]
results = []
def timestamp(value):
    whole,_,fraction = value.rstrip('Z').partition('.')
    return datetime.datetime.strptime(whole,'%Y-%m-%dT%H:%M:%S').replace(
        microsecond=int((fraction+'000000')[:6]),tzinfo=datetime.timezone.utc).timestamp()

for app,mode in cases:
    folder = root/f'{app}-{mode}'
    if '--partial' in sys.argv and not (folder/'summary.json').exists():
        continue
    summary = json.loads((folder/'summary.json').read_text())
    samples = [json.loads(line) for line in (folder/'samples.jsonl').read_text().splitlines()]
    intervals = []
    for previous,current in zip(samples,samples[1:]):
        components = []
        for a,b in zip(previous['containers'],current['containers']):
            delta = (b['cpu_ns']-a['cpu_ns'])/1e9
            start = timestamp(a['cpu_read'])
            end = timestamp(b['cpu_read'])
            assert delta >= 0 and end > start
            components.append({'cpu_seconds':delta,'percent_one_core':delta/(end-start)*100,'seconds':end-start})
        intervals.append({'phase':current['phase'],'components':components})
    result = {'app':app,'mode':mode,'import_seconds':summary['import_seconds'],'phases':{}}
    for phase in ['before','during','after']:
        selected = [row for row in intervals if row['phase']==phase]
        phase_result = {}
        for index,name in enumerate(['application','postgresql','rustfs','whole_stack']):
            if name=='whole_stack':
                cpu = [sum(c['cpu_seconds'] for c in row['components']) for row in selected]
                rates = [sum(c['percent_one_core'] for c in row['components']) for row in selected]
                seconds = [statistics.mean(c['seconds'] for c in row['components']) for row in selected]
            else:
                cpu = [row['components'][index]['cpu_seconds'] for row in selected]
                rates = [row['components'][index]['percent_one_core'] for row in selected]
                seconds = [row['components'][index]['seconds'] for row in selected]
            phase_result[name] = {'cpu_seconds':sum(cpu),'mean_percent_one_core':sum(cpu)/sum(seconds)*100,
                'median_percent_one_core':statistics.median(rates),'peak_percent_one_core':max(rates),
                'sampled_seconds':sum(seconds),'intervals':len(selected),
                'seconds_above_50_percent':sum(s for s,p in zip(seconds,rates) if p>50),
                'seconds_above_100_percent':sum(s for s,p in zip(seconds,rates) if p>100)}
        result['phases'][phase] = phase_result
    results.append(result)
(root/('results-partial.json' if '--partial' in sys.argv else 'results.json')).write_text(json.dumps(results,indent=2))
print(json.dumps(results,indent=2))
