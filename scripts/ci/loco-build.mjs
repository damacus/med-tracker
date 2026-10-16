import { buildArchiveArtifact } from './loco-nextest.mjs';

console.log('Archive the locked Rust workspace and build the application for reuse');
await buildArchiveArtifact();
