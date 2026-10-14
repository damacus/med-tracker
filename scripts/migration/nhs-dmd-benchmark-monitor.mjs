import { execFile } from 'node:child_process';
import { appendFile, readFile } from 'node:fs/promises';
import { cpus, loadavg, totalmem, platform } from 'node:os';
import { basename, join } from 'node:path';
import { promisify } from 'node:util';
import { setTimeout as delay } from 'node:timers/promises';

const execute = promisify(execFile);

export function competingProcesses(rows, testNames = [], processArguments = '') {
  const names = new Set(['rustc', 'cargo', 'rust-lld', 'ld', 'care_api', 'rust_household_browser', 'pytest', 'med_tracker', 'chrome-headless-shell', ...testNames]);
  const playwrightPids = new Set(processArguments.split('\n').map(line => /^\s*(\d+)\s+(.+)$/.exec(line)).filter(Boolean)
    .filter(([, , args]) => /(?:^|\s)(?:\S*\/)?(?:playwright|(?:@playwright\/test|playwright(?:-core)?)\/cli\.js)\s+test(?:\s|$)/.test(args))
    .map(([, pid]) => Number(pid)));
  return rows.filter(row => {
    if (row.name === 'chrome-headless-shell' && Number.isFinite(row.cpuPercent) && row.cpuPercent < 1) return false;
    return names.has(row.name.replace(/-[a-f0-9]{16}$/, '')) || playwrightPids.has(row.pid) || /^(?:npm exec )?playwright test(?: |$)/.test(row.name);
  }).map(row => ({ pid: row.pid, name: row.name }));
}

export function isQuiet(sample) {
  return sample.idlePercent >= 80 && sample.load[0] <= sample.cpuCount
    && sample.memoryPressure === 1 && Number.isFinite(sample.swapouts) && sample.competingProcesses.length === 0;
}

export function quietWindow(samples) {
  if (samples.length < 2) return false;
  return samples.at(-1).timeMs - samples[0].timeMs >= 60000
    && samples.every(isQuiet)
    && samples.every((sample, index) => index === 0 || sample.timeMs - samples[index - 1].timeMs <= 20000)
    && samples.at(-1).swapouts === samples[0].swapouts;
}

async function command(name, args) {
  return (await execute(name, args, { timeout: 10000, maxBuffer: 4 * 1024 * 1024 })).stdout.trim();
}

export async function sampleHost() {
  if (platform() !== 'darwin') throw new Error('This benchmark monitor requires macOS');
  const testNames = JSON.parse(await readFile(new URL('../../target/ci/tests.json', import.meta.url), 'utf8')).map(test => test.name);
  const before = cpus();
  await delay(1000);
  const after = cpus();
  const delta = after.map((cpu, index) => Object.fromEntries(Object.entries(cpu.times).map(([key, value]) => [key, value - before[index].times[key]])));
  const total = delta.flatMap(cpu => Object.values(cpu)).reduce((a, b) => a + b, 0);
  const [pressure, swap, vm, processes, processArguments] = await Promise.all([
    command('sysctl', ['-n', 'kern.memorystatus_vm_pressure_level']),
    command('sysctl', ['-n', 'vm.swapusage']),
    command('vm_stat', []),
    command('ps', ['-axo', 'pid=,pcpu=,rss=,comm=']),
    command('ps', ['-axo', 'pid=,args=']),
  ]);
  const processRows = processes.split('\n').map(line => /^\s*(\d+)\s+([\d.]+)\s+(\d+)\s+(.+)$/.exec(line)).filter(Boolean)
    .map(([, pid, cpu, rss, name]) => ({ pid: Number(pid), cpuPercent: Number(cpu), rssBytes: Number(rss) * 1024, name: basename(name) }));
  const vmCounters = Object.fromEntries([...vm.matchAll(/^([^:\n]+):\s+(\d+)\./gm)].map(([, key, value]) => [key, Number(value)]));
  return {
    observedAt: new Date().toISOString(), timeMs: performance.now(),
    cpuCount: after.length, totalMemoryBytes: totalmem(), load: loadavg(),
    idlePercent: 100 * delta.reduce((sum, cpu) => sum + cpu.idle, 0) / total,
    memoryPressure: Number(pressure), swapUsage: swap,
    swapouts: vmCounters.Swapouts, vmCounters,
    competingProcesses: competingProcesses(processRows, testNames, processArguments),
    topProcesses: processRows.sort((a, b) => b.cpuPercent - a.cpuPercent).slice(0, 15),
  };
}

export class BenchmarkMonitor {
  constructor(directory) {
    this.path = join(directory, 'host-samples.jsonl');
    this.events = join(directory, 'commands.jsonl');
  }

  async sample(stage) {
    const sample = { stage, ...await sampleHost() };
    await appendFile(this.path, `${JSON.stringify(sample)}\n`);
    return sample;
  }

  async waitForQuiet(stage) {
    const started = performance.now();
    let samples = [];
    while (performance.now() - started < 15 * 60 * 1000) {
      const sample = await this.sample(`${stage}:before`);
      if (!isQuiet(sample) || (samples.length && (sample.swapouts !== samples.at(-1).swapouts || sample.timeMs - samples.at(-1).timeMs > 20000))) samples = [];
      if (isQuiet(sample)) samples.push(sample);
      process.stdout.write(`${stage} quiet gate: idle=${sample.idlePercent.toFixed(1)}%, load=${sample.load[0].toFixed(2)}, pressure=${sample.memoryPressure}, competing=${sample.competingProcesses.length}, stable=${samples.length ? Math.floor((sample.timeMs - samples[0].timeMs) / 1000) : 0}s\n`);
      if (quietWindow(samples)) return;
      await delay(9000);
    }
    throw new Error(`No verified quiet minute before ${stage}; import was not started`);
  }

  async measure(stage, operation, { quiet = false } = {}) {
    if (quiet) await this.waitForQuiet(stage);
    const startedAt = new Date().toISOString();
    const started = performance.now();
    let finished = false;
    let samplingError;
    const sampling = (async () => {
      while (!finished) {
        try { await this.sample(stage); } catch (error) { samplingError = error; return; }
        for (let i = 0; i < 9 && !finished; i++) await delay(1000);
      }
    })();
    let outcome = 'failure';
    try {
      const result = await operation();
      outcome = 'success';
      return result;
    } finally {
      const durationMs = performance.now() - started;
      finished = true;
      await sampling;
      await appendFile(this.events, `${JSON.stringify({ stage, startedAt, durationMs, outcome, samplingError: samplingError?.message ?? null })}\n`);
      if (samplingError && outcome === 'success') throw samplingError;
    }
  }
}
