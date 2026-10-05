const delay = milliseconds => new Promise(resolve => setTimeout(resolve, milliseconds));

export async function stopOwnedProcess(child, { termMs = 2000, killMs = 2000 } = {}) {
  if (!Number.isInteger(child.pid)) return;
  const group = -child.pid;
  const alive = () => {
    try { process.kill(group, 0); return true; } catch (error) {
      if (error.code === 'ESRCH') return false;
      throw error;
    }
  };
  const signal = name => {
    try { process.kill(group, name); } catch (error) { if (error.code !== 'ESRCH') throw error; }
  };
  const wait = async milliseconds => {
    const deadline = Date.now() + milliseconds;
    while (alive() && Date.now() < deadline) await delay(20);
    return !alive();
  };
  if (alive()) {
    signal('SIGTERM');
    if (!await wait(termMs)) {
      signal('SIGKILL');
      if (!await wait(killMs)) throw new Error(`Owned process group ${child.pid} survived SIGKILL`);
    }
  }
  const deadline = Date.now() + killMs;
  while (child.exitCode === null && child.signalCode === null && Date.now() < deadline) await delay(20);
  if (child.exitCode === null && child.signalCode === null) throw new Error(`Owned child ${child.pid} did not report exit`);
}
