// Temporary private profiler sampler. No command lines, arguments or clinical data are read.
import fs from 'node:fs';
const [directory, cgroup, postmasterText, container] = process.argv.slice(2);
const postmaster = Number(postmasterText);
if (!/^[0-9a-f]{64}$/.test(container) || !Number.isSafeInteger(postmaster) || postmaster <= 0 || !cgroup.startsWith('/sys/fs/cgroup/') || !cgroup.includes(container)) process.exit(1);
const pgExe = fs.statSync(`/proc/${postmaster}/exe`);
const identity = stat => `${stat.dev}:${stat.ino}`;
const pgIdentity = identity(pgExe);
const output = fs.openSync(`${directory}/snapshots.jsonl`, 'wx', 0o600);
let rootStart;
function sample() {
  const before = process.hrtime.bigint();
  const utc = BigInt(Date.now()) * 1000000n;
  const after = process.hrtime.bigint();
  const processes = new Map();
  let unverified = 0;
  for (const pidText of fs.readFileSync(`${cgroup}/cgroup.procs`, 'utf8').trim().split(/\s+/)) {
    if (!/^\d+$/.test(pidText)) { unverified++; continue; }
    try {
      const base = `/proc/${pidText}`;
      if (identity(fs.statSync(`${base}/exe`)) !== pgIdentity) continue;
      const status = fs.readFileSync(`${base}/status`, 'utf8');
      const ppid = status.match(/^PPid:\s+(\d+)$/m)?.[1];
      const ns = status.match(/^NSpid:\s+([\d\s]+)$/m)?.[1].trim().split(/\s+/).at(-1);
      const stat = fs.readFileSync(`${base}/stat`, 'utf8');
      const close = stat.lastIndexOf(') ');
      const start = close < 0 ? null : stat.slice(close + 2).split(' ')[19];
      const membership = fs.readFileSync(`${base}/cgroup`, 'utf8');
      if (!ppid || !ns || !/^\d+$/.test(start || '') || !membership.includes(container)) { unverified++; continue; }
      processes.set(pidText, {pid:pidText, ppid, ns, start, exe:pgIdentity});
    } catch (error) {
      // A disappearing process cannot be established as a PostgreSQL descendant.
      unverified++;
    }
  }
  const selected = [];
  for (const p of processes.values()) {
    let cursor = p.pid, depth = 0;
    while (cursor !== postmasterText && processes.has(cursor) && depth++ < 32) cursor = processes.get(cursor).ppid;
    if (cursor === postmasterText) selected.push(p); else unverified++;
  }
  const root = processes.get(postmasterText);
  if (!root || (rootStart && root.start !== rootStart)) throw new Error('postmaster identity unavailable');
  rootStart = root.start;
  const cpu = Object.fromEntries(fs.readFileSync(`${cgroup}/cpu.stat`, 'utf8').trim().split('\n').map(line => line.trim().split(/\s+/)));
  const host = fs.readFileSync('/proc/stat', 'utf8').split('\n')[0].trim().split(/\s+/);
  if (host.shift() !== 'cpu' || host.length < 8 || host.slice(0,8).some(value => !/^\d+$/.test(value))) throw new Error('host CPU counters unavailable');
  const observedUntil = process.hrtime.bigint();
  fs.writeSync(output, JSON.stringify({before:before.toString(), utc:utc.toString(), after:after.toString(), observed_until:observedUntil.toString(), unverified, cpu, host:host.slice(0,8), processes:selected}) + '\n');
}
try {
  sample();
  fs.writeFileSync(`${directory}/sampler.ready`, 'ready\n', {mode:0o600, flag:'wx'});
  const timer = setInterval(() => {
    try {
      if (fs.existsSync(`${directory}/sampler.stop`)) { clearInterval(timer); fs.closeSync(output); return; }
      sample();
    } catch (_) { clearInterval(timer); fs.closeSync(output); process.exitCode = 1; }
  }, 250);
} catch (_) { fs.closeSync(output); process.exitCode = 1; }
