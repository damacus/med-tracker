import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { randomUUID } from 'node:crypto';
import { withOwnedDatabase } from '../../scripts/migration/foundation-database.mjs';
import { runOwnedTemplateTask } from './template_task_process.mjs';

const directory = await mkdtemp(join(tmpdir(), 'medtracker-template-concurrency-'));
try {
  await withOwnedDatabase(async (url, provision, ownership) => {
    process.stdout.write(`Template regression project: ${ownership.project}\n`);
    await provision();
    const suffix = [...randomUUID().replaceAll('-', '')]
      .map((digit) => String.fromCharCode(97 + Number.parseInt(digit, 16)))
      .join('');
    const name = `template_guard_${suffix}`;
    await runOwnedTemplateTask('task', ['test:template-concurrency:child'], {
      cwd: process.cwd(),
      env: { ...process.env, DATABASE_URL: url, MEDTRACKER_OWNED_DATABASE: '1', TEMPLATE_REGRESSION_NAME: name,
        TEMPLATE_REGRESSION_MARKER: join(directory, 'seeded') },
    });
  });
  process.stdout.write('Template regression project cleaned up\n');
} finally {
  await rm(directory, { recursive: true, force: true });
}
