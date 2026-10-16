import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { createNextestProofReport, parseNextestJUnit, parseNextestJUnitSuites } from './nextest-proof.mjs';

const suite = (cases, counts = {}) => `<?xml version="1.0" encoding="UTF-8"?>
<testsuites name="nextest-run" tests="${counts.tests ?? cases.length}" failures="${counts.failures ?? 0}" errors="${counts.errors ?? 0}">
<testsuite name="med-tracker::requests" tests="${counts.tests ?? cases.length}" disabled="0" failures="${counts.failures ?? 0}" errors="${counts.errors ?? 0}">${cases.join('')}</testsuite></testsuites>`;
const passed = (name) => `<testcase name="${name}" classname="med-tracker::requests" time="0.001"/>`;

test('credits exact executed names from the expected binary', () => {
  const proof = parseNextestJUnit(suite([passed('api_v1::baseline::anonymous_access'), passed('api_v1::acceptance')]), 'med-tracker::requests');
  assert.deepEqual([...proof.passed], ['api_v1::baseline::anonymous_access', 'api_v1::acceptance']);
  assert.deepEqual([...proof.ignored], []);
});

test('keeps ignored cases separate from executed credit', () => {
  const proof = parseNextestJUnit(suite([passed('api_v1::baseline::anonymous_access'), '<testcase name="api_v1::acceptance" classname="med-tracker::requests"><skipped/></testcase>']), 'med-tracker::requests');
  assert.deepEqual([...proof.passed], ['api_v1::baseline::anonymous_access']);
  assert.deepEqual([...proof.ignored], ['api_v1::acceptance']);
});

test('multi-binary JUnit proof credits each expected suite independently', () => {
  const xml = '<testsuites tests="2" failures="0" errors="0"><testsuite name="med-tracker::requests" tests="1" failures="0" errors="0"><testcase name="api_v1::acceptance" classname="med-tracker::requests"/></testsuite><testsuite name="migration::identity_schema" tests="1" failures="0" errors="0"><testcase name="structural_identity" classname="migration::identity_schema"/></testsuite></testsuites>';
  const proof = parseNextestJUnitSuites(xml, ['med-tracker::requests', 'migration::identity_schema']);
  assert.deepEqual([...proof.get('med-tracker::requests').passed], ['api_v1::acceptance']);
  assert.deepEqual([...proof.get('migration::identity_schema').passed], ['structural_identity']);
  for (const invalid of [xml.replace('migration::identity_schema', 'wrong::binary'), xml.replace('tests="2"', 'tests="3"'), xml.replace('</testsuites>', '<testsuite name="med-tracker::requests" tests="0" failures="0" errors="0"/></testsuites>')]) {
    assert.throws(() => parseNextestJUnitSuites(invalid, ['med-tracker::requests', 'migration::identity_schema']));
  }
  assert.throws(() => parseNextestJUnit(xml, 'med-tracker::requests'));
});

for (const [label, xml] of [
  ['zero', suite([])],
  ['missing', ''],
  ['malformed', '<testsuites><testsuite>'],
  ['doctype', '<!DOCTYPE testsuites [<!ENTITY x SYSTEM "file:///etc/passwd">]>' + suite([passed('safe')])],
  ['duplicate', suite([passed('api_v1::acceptance'), passed('api_v1::acceptance')])],
  ['duplicate skip', suite([passed('safe'), '<testcase name="api_v1::acceptance" classname="med-tracker::requests"><skipped/><skipped/></testcase>'])],
  ['failed', suite(['<testcase name="api_v1::acceptance" classname="med-tracker::requests"><failure/></testcase>'], { failures: 1 })],
  ['errored', suite(['<testcase name="api_v1::acceptance" classname="med-tracker::requests"><error/></testcase>'], { errors: 1 })],
  ['skipped', suite(['<testcase name="api_v1::acceptance" classname="med-tracker::requests"><skipped/></testcase>'])],
  ['wrong binary', suite([passed('api_v1::acceptance')]).replaceAll('med-tracker::requests', 'migration::identity_schema')],
  ['count mismatch', suite([passed('api_v1::acceptance')], { tests: 2 })],
  ['extra suite', suite([passed('api_v1::acceptance')]).replace('</testsuites>', '<testsuite name="other" tests="0" failures="0" errors="0"/></testsuites>')],
]) test(`rejects ${label} JUnit proof`, () => {
  assert.throws(() => parseNextestJUnit(xml, 'med-tracker::requests'));
});

test('proof invocations use unique report paths and reject an absent fresh report', async () => {
  const target = await mkdtemp(join(tmpdir(), 'medtracker-nextest-target-'));
  const first = await createNextestProofReport(target);
  const second = await createNextestProofReport(target);
  try {
    assert.notEqual(first.reportPath, second.reportPath);
    assert.match(await readFile(first.configPath, 'utf8'), /report-skipped = "ignored"/);
    await writeFile(first.reportPath, suite([passed('api_v1::acceptance')]));
    await assert.rejects(second.read('med-tracker::requests'), /missing|report/i);
    assert.deepEqual([...(await first.read('med-tracker::requests')).passed], ['api_v1::acceptance']);
  } finally {
    await first.cleanup();
    await second.cleanup();
    await rm(target, { recursive: true, force: true });
  }
});
