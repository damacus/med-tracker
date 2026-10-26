import csv
import json
import math
import statistics
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

root = Path('/tmp/medtracker-benchmark')
cases = [('rails','fork'),('rails','async'),('rails','inline'),('loco','async'),('loco','inline')]
records = []
fig, axes = plt.subplots(3, 2, figsize=(12, 11), sharey=True)
maximum = 0
for (app,mode), ax in zip(cases,axes.flat):
    folder = root/f'{app}-{mode}'
    summary = json.loads((folder/'summary.json').read_text())
    samples = [json.loads(line) for line in (folder/'samples.jsonl').read_text().splitlines()]
    probes = [json.loads(line) for line in (folder/'probes.jsonl').read_text().splitlines()]
    name = f'mt-idle-6577-{app}-1'
    phases = summary['phases']
    during = [row for row in samples if row['phase']=='during']
    after = [row for row in samples if row['phase']=='after']
    origin = during[0]['elapsed_seconds']
    after_time = after[0]['elapsed_seconds']
    immediate = [row['containers'][0]['working_bytes']/1048576 for row in after if row['elapsed_seconds']<after_time+10]
    responsive = [row for row in probes if row['phase']=='during' and 'dashboard' in row['path']]
    delays = sorted(row['seconds'] for row in responsive)
    record = {'stack':app,'mode':mode,
        'before_MiB':phases['before'][name]['median_MiB'],
        'peak_during_MiB':phases['during'][name]['peak_MiB'],
        'immediate_after_MiB':statistics.median(immediate),
        'after_5_minutes_MiB':phases['after'][name]['final_minute_median_MiB'],
        'anon_after_5_minutes_MiB':phases['after'][name]['final_minute_anon_MiB'],
        'anon_before_MiB':phases['before'][name]['final_minute_anon_MiB'],
        'stack_before_MiB':statistics.median(sum(c['working_bytes'] for c in row['containers'])/1048576 for row in samples if row['phase']=='before'),
        'stack_peak_during_MiB':max(sum(c['working_bytes'] for c in row['containers'])/1048576 for row in during),
        'stack_after_5_minutes_MiB':statistics.median(sum(c['working_bytes'] for c in row['containers'])/1048576 for row in after if row['elapsed_seconds']>=after[-1]['elapsed_seconds']-60),
        'import_seconds':summary['import_seconds'],
        'completed':summary['import']['status']==4,
        'imported':summary['import']['imported_count'],
        'skipped':summary['import']['skipped_count'],
        'dashboard_probes_during':len(responsive),
        'dashboard_failures_during':sum('error' in row or row.get('status')!=200 for row in responsive),
        'dashboard_p95_seconds':delays[max(0,math.ceil(len(delays)*.95)-1)] if delays else None}
    records.append(record)
    x = [(row['elapsed_seconds']-origin)/60 for row in samples]
    y = [row['containers'][0]['working_bytes']/1048576 for row in samples]
    maximum = max(maximum,max(y))
    ax.axvspan(-1,0,color='#edf0f3')
    ax.axvspan(0,(after_time-origin)/60,color='#fce6d3')
    ax.axvspan((after_time-origin)/60,x[-1],color='#e6f3ed')
    ax.plot(x,y,color='#174873',linewidth=1.6)
    title = {'fork':'standard forked jobs','async':'threaded background job' if app=='rails' else 'background job','inline':'inline request'}[mode]
    ax.set_title(('Rails' if app=='rails' else 'Rust / Loco')+' — '+title,loc='left',fontweight='bold')
    ax.set_xlabel('Minutes from upload')
    ax.set_ylabel('App container memory (MiB)')
    ax.grid(axis='y',alpha=.22)
    ax.spines[['top','right']].set_visible(False)
    ax.text(.98,.95,f'Peak {record["peak_during_MiB"]:.0f} MiB\nAfter 5 min {record["after_5_minutes_MiB"]:.0f} MiB',
            ha='right',va='top',transform=ax.transAxes,fontsize=10)
for ax in axes.flat:
    ax.set_ylim(0,maximum*1.15)
axes.flat[-1].set_visible(False)
fig.suptitle('DM+d import: memory before, during and after',x=.07,ha='left',fontweight='bold',fontsize=17)
fig.text(.07,.925,'Grey: before upload   ·   Orange: import   ·   Green: five minutes after completion',fontsize=10,color='#454b53')
fig.text(.07,.025,'Single app container per stack. Same real release and fresh seeded database per trial. No forced GC or restart.\nDocker working-set memory excludes inactive file cache; PostgreSQL and RustFS are measured separately.',fontsize=9,color='#454b53')
fig.tight_layout(rect=[0,.065,1,.90])
fig.savefig(root/'memory-import.png',dpi=160)
fig.savefig(root/'memory-import.pdf')
(root/'results.json').write_text(json.dumps(records,indent=2))
with (root/'results.csv').open('w') as file:
    writer=csv.DictWriter(file,fieldnames=list(records[0]))
    writer.writeheader(); writer.writerows(records)
print(json.dumps(records,indent=2))
