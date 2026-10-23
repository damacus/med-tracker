import concurrent.futures
import datetime
import http.client
import json
import socket
import statistics
import subprocess
import sys
import threading
import time
from pathlib import Path
from client import Client

ROOT = Path('/tmp/medtracker-cpu-benchmark')
app, mode = sys.argv[1:3]
trial = {"async":"a","inline":"b","fork":"c"}[mode]
label = f'{app}-{mode}'
out = ROOT / label
out.mkdir(exist_ok=False)
started_wall = datetime.datetime.now(datetime.timezone.utc).isoformat()

class DockerConnection(http.client.HTTPConnection):
    def __init__(self):
        super().__init__('localhost', timeout=10)
    def connect(self):
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.sock.settimeout(self.timeout)
        self.sock.connect('/var/run/docker.sock')

def api(path):
    conn = DockerConnection()
    try:
        conn.request('GET', path)
        response = conn.getresponse()
        payload = response.read()
        if response.status != 200:
            raise RuntimeError((response.status, payload[:200]))
        return json.loads(payload)
    finally:
        conn.close()

names = [f'mt-idle-6577-{name}-1' for name in [app, app+'-db', app+'-storage']]
identities = [api('/containers/'+name+'/json') for name in names]
(out/'containers.json').write_text(json.dumps([
    {'name':row['Name'], 'id':row['Id'], 'image':row['Image'],
     'command':row['Config']['Cmd'], 'memory_limit':row['HostConfig']['Memory'],
     'started':row['State']['StartedAt']} for row in identities], indent=2))

client = Client(app)
for attempt in range(60):
    try:
        status, _, _ = client.fetch('/up' if app == 'rails' else '/health', timeout=2)
        if status == 200:
            break
    except Exception:
        pass
    time.sleep(1)
else:
    raise RuntimeError('Application did not become healthy')
client.login()
admin_form = '/households/persistence-fixture/admin/nhs_dmd_import/new' if app == 'rails' else '/admin/nhs-dmd'
status, _, _ = client.fetch(admin_form)
assert status == 200, ('admin access preflight', status)

current = {'phase': 'before'}
stop = threading.Event()
probes = []
def probe_loop():
    next_dashboard = 0
    with (out/'probes.jsonl').open('w') as file:
        while not stop.is_set():
            now = time.monotonic()
            path = '/households/persistence-fixture/dashboard' if now >= next_dashboard else ('/up' if app == 'rails' else '/health')
            if 'dashboard' in path:
                next_dashboard = now + 5
            tick = time.monotonic()
            row = {'time': time.time(), 'phase': current['phase'], 'path': path}
            try:
                code, _, _ = client.fetch(path, timeout=2)
                row['status'] = code
            except Exception as error:
                row['error'] = type(error).__name__ + ': ' + str(error)
            row['seconds'] = time.monotonic() - tick
            probes.append(row)
            file.write(json.dumps(row)+'\n'); file.flush()
            stop.wait(max(0, 1 - row['seconds']))

probe_thread = threading.Thread(target=probe_loop, daemon=True)
probe_thread.start()
samples = []
latest = None
terminal_at = None
upload_future = None
upload_status = None
upload_error = None
before_start = time.monotonic()
import_start = None
next_poll = 0
next_print = 0
pool = concurrent.futures.ThreadPoolExecutor(max_workers=4)

def stats(name):
    data = api('/containers/'+name+'/stats?stream=false&one-shot=true')
    memory = data['memory_stats']
    details = memory.get('stats', {})
    raw = memory.get('usage', 0)
    return {'name': name, 'raw_bytes': raw,
            'working_bytes': max(0, raw-details.get('inactive_file', details.get('total_inactive_file',0))),
            'anon_bytes': details.get('anon',details.get('rss',0)),
            'file_bytes': details.get('file',details.get('cache',0)),
            'inactive_file_bytes':details.get('inactive_file',0),
            'pids':data.get('pids_stats',{}).get('current'), 'cpu_ns':data['cpu_stats']['cpu_usage']['total_usage'], 'cpu_read':data['read']}

def progress():
    sql = 'SELECT row_to_json(s) FROM (SELECT status,total_records,processed_records,imported_count,created_count,updated_count,unchanged_count,skipped_count,error_message FROM nhs_dmd_imports ORDER BY id DESC LIMIT 1) s'
    result = subprocess.run(['rtk','proxy','docker','exec',f'mt-idle-6577-{app}-db-1',
        'psql','-U','medtracker','-d',f'cpu_trial_{trial}','-Atc',sql], capture_output=True,text=True,timeout=15,check=True)
    return json.loads(result.stdout) if result.stdout.strip() else None

try:
    with (out/'samples.jsonl').open('w') as file:
        while True:
            tick = time.monotonic()
            phase = current['phase']
            rows = list(pool.map(stats, names))
            if phase == 'during' and tick >= next_poll:
                latest = progress()
                next_poll = tick + 5
                if latest and latest['status'] in [4,5] and upload_future.done():
                    try:
                        upload_status = upload_future.result()
                    except Exception as error:
                        upload_error = repr(error)
                    terminal_at = time.monotonic()
                    current['phase'] = 'after'
            if upload_future and upload_future.done() and phase == 'during' and latest is None:
                try:
                    upload_status = upload_future.result()
                except Exception as error:
                    upload_error = repr(error)
                    terminal_at = time.monotonic()
                    current['phase'] = 'after'
            row = {'time':time.time(),'elapsed_seconds':tick-before_start,'phase':phase,
                   'containers':rows,'import':latest}
            samples.append(row)
            file.write(json.dumps(row)+'\n'); file.flush()
            if tick >= next_print:
                print(json.dumps({'trial':label,'phase':phase,'elapsed_seconds':round(tick-before_start),
                    'app_MiB':round(rows[0]['working_bytes']/1048576,1),'import':latest}),flush=True)
                next_print = tick+30
            state = api('/containers/'+names[0]+'/json')['State']
            if not state['Running']:
                raise RuntimeError('Application stopped: '+json.dumps(state))
            if phase == 'before' and tick-before_start >= 60:
                current['phase'] = 'during'
                import_start = time.monotonic()
                upload_future = pool.submit(client.upload, ROOT/'release.zip')
            elif phase == 'during' and tick-import_start > 7200:
                raise RuntimeError('Import exceeded two-hour observation limit')
            elif phase == 'after' and tick-terminal_at >= 60:
                break
            time.sleep(max(0,0.5-(time.monotonic()-tick)))
finally:
    stop.set()
    probe_thread.join(timeout=3)

summary = {'app':app,'mode':mode,'started':started_wall,'import':latest,
           'upload_http_status':upload_status,'upload_error':upload_error,
           'import_seconds':terminal_at-import_start if terminal_at else None, 'import_start_elapsed':import_start-before_start, 'import_end_elapsed':terminal_at-before_start,
           'phases':{}, 'probes':{}}
for phase in ['before','during','after']:
    selected = [row for row in samples if row['phase']==phase]
    if phase == 'after' and selected:
        cutoff = selected[-1]['elapsed_seconds']-60
        retained = [row for row in selected if row['elapsed_seconds']>=cutoff]
    else:
        retained = selected
    summary['phases'][phase] = {}
    for index,name in enumerate(names):
        values = [row['containers'][index]['working_bytes']/1048576 for row in selected]
        tail = [row['containers'][index]['working_bytes']/1048576 for row in retained]
        anon = [row['containers'][index]['anon_bytes']/1048576 for row in retained]
        summary['phases'][phase][name] = {'median_MiB':statistics.median(values),
            'peak_MiB':max(values),'final_minute_median_MiB':statistics.median(tail),
            'final_minute_anon_MiB':statistics.median(anon),'samples':len(values)}
    selected_probes = [row for row in probes if row['phase']==phase]
    summary['probes'][phase] = {'requests':len(selected_probes),
        'errors':sum('error' in row or row.get('status')!=200 for row in selected_probes),
        'max_seconds':max((row['seconds'] for row in selected_probes),default=0),
        'median_seconds':statistics.median([row['seconds'] for row in selected_probes]) if selected_probes else None}
(out/'summary.json').write_text(json.dumps(summary,indent=2))
print(json.dumps(summary,indent=2),flush=True)
