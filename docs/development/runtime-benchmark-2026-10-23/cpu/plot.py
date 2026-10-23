import json
import datetime
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

root=Path('/tmp/medtracker-cpu-benchmark')
rows=json.loads((root/'results.json').read_text())
fig,axes=plt.subplots(3,2,figsize=(12,10))
labels={'async':'background','inline':'inline','fork':'standard fork'}
def timestamp(value):
    whole,_,fraction=value.rstrip('Z').partition('.')
    return datetime.datetime.strptime(whole,'%Y-%m-%dT%H:%M:%S').replace(
        microsecond=int((fraction+'000000')[:6]),tzinfo=datetime.timezone.utc).timestamp()

for result,ax in zip(rows,axes.flat):
    app,mode=result['app'],result['mode']
    samples=[json.loads(line) for line in (root/f'{app}-{mode}'/'samples.jsonl').read_text().splitlines()]
    origin=json.loads((root/f'{app}-{mode}'/'summary.json').read_text())['import_start_elapsed']
    x=[]; app_rates=[]; stack_rates=[]
    for a,b in zip(samples,samples[1:]):
        rates=[(new['cpu_ns']-old['cpu_ns'])/1e9/(timestamp(new['cpu_read'])-timestamp(old['cpu_read']))*100
               for old,new in zip(a['containers'],b['containers'])]
        x.append((b['elapsed_seconds']-origin)/60)
        app_rates.append(rates[0]); stack_rates.append(sum(rates))
    end=result['import_seconds']/60
    ax.axvspan(-1,0,color='#edf0f3')
    ax.axvspan(0,end,color='#fce6d3')
    ax.axvspan(end,x[-1],color='#e6f3ed')
    ax.plot(x,stack_rates,color='#999999',linewidth=.9,label='Whole stack')
    ax.plot(x,app_rates,color='#174873',linewidth=1,label='App')
    ax.set_title(('Rails' if app=='rails' else 'Loco')+' — '+labels[mode],loc='left',fontweight='bold')
    ax.set_xlabel('Minutes from upload'); ax.set_ylabel('CPU (% of one core)')
    ax.set_ylim(bottom=0); ax.grid(axis='y',alpha=.2)
    ax.spines[['top','right']].set_visible(False)
    ax.legend(loc='upper right',fontsize=8)
ax=axes.flat[-1]
names=[('Rails' if r['app']=='rails' else 'Loco')+' '+labels[r['mode']] for r in rows]
left=[0]*len(rows)
for key,label,color in [('application','App','#174873'),('postgresql','PostgreSQL','#e78c3c'),('rustfs','RustFS','#7ab39a')]:
    values=[r['phases']['during'][key]['cpu_seconds'] for r in rows]
    ax.barh(names,values,left=left,label=label,color=color)
    left=[a+b for a,b in zip(left,values)]
ax.invert_yaxis(); ax.set_xlabel('CPU-seconds during observed import')
ax.set_title('Total CPU work per equivalent import',loc='left',fontweight='bold')
ax.legend(fontsize=8); ax.spines[['top','right']].set_visible(False)
fig.suptitle('DM+d import: CPU bursts and total CPU work',x=.07,ha='left',fontsize=16,fontweight='bold')
fig.text(.07,.925,'Separate single-run CPU follow-up. Grey: before · Orange: import · Green: one minute after.',fontsize=10)
fig.text(.07,.02,'100% = one busy logical CPU. Rates average roughly half-second intervals; shorter peaks can be missed.\nSame complete dataset and verified catalogue output. CPU-seconds measure work, not electricity or hardware capacity.',fontsize=9)
fig.tight_layout(rect=[0,.065,1,.90])
fig.savefig(root/'cpu-import.png',dpi=160)
