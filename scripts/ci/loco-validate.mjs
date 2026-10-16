import { validateArchiveArtifact } from './loco-nextest.mjs';
import { sourceIdentity } from '../migration/run-slice.mjs';

await validateArchiveArtifact('target/ci', await sourceIdentity(process.env));
