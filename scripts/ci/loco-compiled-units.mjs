import { runArchiveStage } from './loco-nextest.mjs';

console.log('Database-free compiled contract tests from the nextest archive');
await runArchiveStage({ stage: 'unit' });
