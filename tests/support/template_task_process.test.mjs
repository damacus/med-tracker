import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { withOwnedDatabase } from '../../scripts/migration/foundation-database.mjs';
import { runOwnedTemplateTask } from './template_task_process.mjs';

const delay = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds));

async function expectGone(pid) {
  for (let attempt = 0; attempt < 100; attempt += 1) {
    try { process.kill(pid, 0); } catch (error) {
      if (error.code === 'ESRCH') return;
      throw error;
    }
    await delay(20);
  }
  assert.fail(`owned descendant ${pid} remained alive`);
}

for (const mode of ['hang', 'early-exit']) {
  test(`owned template task cleans ${mode} descendants before database teardown`, async () => {
    const directory = await mkdtemp(join(tmpdir(), 'medtracker-template-child-'));
    const pidFile = join(directory, 'descendant.pid');
    const databaseCalls = [];
    try {
      const script = `const {spawn}=require('node:child_process'); const {writeFileSync}=require('node:fs');
        const child=spawn(process.execPath,['-e','setInterval(()=>{},1000)'],{stdio:'ignore'});
        writeFileSync(process.env.DESCENDANT_PID_FILE,String(child.pid));
        if(process.env.CHILD_MODE==='early-exit') process.exit(7);
        setInterval(()=>{},1000);`;
      const run = () => withOwnedDatabase(async () => {
        await runOwnedTemplateTask(process.execPath, ['-e', script], {
          env: { ...process.env, DESCENDANT_PID_FILE: pidFile, CHILD_MODE: mode },
          timeoutMs: 300,
          spawnProcess: spawn,
        });
      }, async (args) => {
        databaseCalls.push(args[0]);
        if (args[0] === 'foundation:db-port') return '127.0.0.1:54321';
        if (args[0] === 'foundation:db-down') await expectGone(Number(await readFile(pidFile, 'utf8')));
        return '';
      }, {});
      await assert.rejects(run, mode === 'hang' ? /exceeded 300ms/ : /exited 7/);
      assert.deepEqual(databaseCalls, ['foundation:db-up', 'foundation:db-port', 'foundation:db-down']);
      await expectGone(Number(await readFile(pidFile, 'utf8')));
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });
}
