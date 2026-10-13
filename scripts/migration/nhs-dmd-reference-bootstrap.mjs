export async function runReferenceImport({ url, provision, runCommand, locoArguments, locoEnvironment, railsArguments, environment }) {
  const referenceUrl = new URL(url);
  if (referenceUrl.hostname !== '127.0.0.1' || referenceUrl.pathname !== '/medtracker_loco' || !referenceUrl.port) throw new Error('Reference import requires an owned loopback database');
  referenceUrl.pathname = '/medtracker_reference';
  await provision();
  await runCommand('task', ['db:migrate', `MIGRATION_DATABASE_URL=${referenceUrl.href}`, 'LOCO_ENV=test'], environment);
  await runCommand('cargo', locoArguments, locoEnvironment ?? environment);
  await runCommand('docker', railsArguments, environment);
}
