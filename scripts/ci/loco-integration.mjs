import { runArchiveStage } from './loco-nextest.mjs';

await runArchiveStage({ stage: 'integration', shard: process.argv[2] ?? '1/1' });
