// Temporary private analysis; writes PID selections only inside the private work directory.
import fs from 'node:fs';
const [directory, markerPath] = process.argv.slice(2);
const markerText = fs.readFileSync(markerPath, 'utf8');
if (!/^\d{19}$/.test(markerText)) process.exit(1);
const marker = BigInt(markerText);
const entries = fs.readFileSync(`${directory}/snapshots.jsonl`, 'utf8').trim().split('\n').map(line => JSON.parse(line));
const earlier = entries.filter(x => BigInt(x.utc) <= marker).at(-1);
const later = entries.find(x => BigInt(x.utc) > marker);
if (!earlier || !later || entries.indexOf(earlier) >= entries.indexOf(later)) process.exit(1);
function offset(x) { return [BigInt(x.utc)-BigInt(x.after), BigInt(x.utc)+1000000n-BigInt(x.before)]; }
const a = offset(earlier), b = offset(later);
const lower = a[0] > b[0] ? a[0] : b[0];
const upper = a[1] < b[1] ? a[1] : b[1];
if (upper < lower || upper-lower > 10000000n) process.exit(1);
const centre = marker-(lower+upper)/2n;
const lo=centre-4000000000n, hi=centre-1000000000n;
const before = entries.filter(x => BigInt(x.observed_until) <= lo).at(-1);
const after = entries.find(x => BigInt(x.before) >= hi);
if (!before || !after) process.exit(1);
const selected = entries.slice(entries.indexOf(before), entries.indexOf(after)+1);
if (selected.length < 2) process.exit(1);
let combinedLower=lower, combinedUpper=upper;
for (const row of selected) {
  const pair=offset(row);
  if(pair[0]>combinedLower) combinedLower=pair[0];
  if(pair[1]<combinedUpper) combinedUpper=pair[1];
}
if(combinedUpper<combinedLower) process.exit(1);
for (let i=1;i<selected.length;i++) if(BigInt(selected[i].before)-BigInt(selected[i-1].observed_until)>1000000000n) process.exit(1);
const identity = p => `${p.pid}:${p.ppid}:${p.start}:${p.ns}:${p.exe}`;
let stable = new Map(before.processes.map(p => [identity(p), p.pid]));
for (const row of selected.slice(1)) {
  const present = new Set(row.processes.map(identity));
  stable = new Map([...stable].filter(([key]) => present.has(key)));
}
if (!stable.size) process.exit(1);
const known = new Set(stable.values());
const loSeconds = Number(lo)/1e9, hiSeconds = Number(hi)/1e9;
const counts = new Map(); let total=0, selectedCount=0, totalPeriod=0n, selectedPeriod=0n;
for (const line of fs.readFileSync(`${directory}/samples`, 'utf8').split('\n')) {
  if (!line.trim()) continue;
  const m=line.trim().match(/^(\d+)\s+(\d+(?:\.\d+)?):?\s+(\d+)$/); if(!m) process.exit(1);
  const time=Number(m[2]); if(time<loSeconds || time>hiSeconds) continue;
  const period=BigInt(m[3]); total++; totalPeriod+=period;
  if(known.has(m[1])) { selectedCount++; selectedPeriod+=period; counts.set(m[1],(counts.get(m[1])||0)+1); }
}
const sampled=[...counts.entries()].sort((x,y)=>y[1]-x[1]);
if (!total || sampled.length<2) process.exit(1);
const cpu = {};
for (const key of ['usage_usec','user_usec','system_usec','throttled_usec']) {
  if (!/^\d+$/.test(before.cpu[key] || '') || !/^\d+$/.test(after.cpu[key] || '')) process.exit(1);
  const delta=BigInt(after.cpu[key])-BigInt(before.cpu[key]); if(delta<0n) process.exit(1); cpu[key]=delta.toString();
}
if (!Array.isArray(before.host) || !Array.isArray(after.host) || before.host.length!==8 || after.host.length!==8) process.exit(1);
const host = before.host.map((value,i) => { if(!/^\d+$/.test(value) || !/^\d+$/.test(after.host[i])) process.exit(1); const delta=BigInt(after.host[i])-BigInt(value); if(delta<0n) process.exit(1); return delta.toString(); });
const info = {scope:'verified_pid_subset',clock:'monotonic',start:loSeconds.toFixed(9),end:hiSeconds.toFixed(9),calibration_uncertainty_ns:(upper-lower).toString(),cpu_observation_start_ns:before.before,cpu_observation_end_ns:after.observed_until,snapshot_unverified:selected.reduce((sum,x)=>sum+x.unverified,0),total,selected:selectedCount,excluded:total-selectedCount,total_period:totalPeriod.toString(),selected_period:selectedPeriod.toString(),a:sampled[0][1],b:sampled[1][1],cpu,host};
fs.writeFileSync(`${directory}/scope.json`, JSON.stringify(info), {mode:0o600});
fs.writeFileSync(`${directory}/pids`, [...known].join(','), {mode:0o600});
fs.writeFileSync(`${directory}/a`, sampled[0][0], {mode:0o600});
fs.writeFileSync(`${directory}/b`, sampled[1][0], {mode:0o600});
