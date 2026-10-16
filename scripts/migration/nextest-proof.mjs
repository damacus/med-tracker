import { SaxesParser } from 'saxes';
import { randomUUID } from 'node:crypto';
import { mkdtemp, mkdir, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, join } from 'node:path';

const MAX_REPORT_BYTES = 1024 * 1024;

export async function createNextestProofReport(targetDirectory) {
  const directory = await mkdtemp(join(tmpdir(), 'medtracker-nextest-proof-'));
  const reportPath = join(targetDirectory, 'nextest', 'contract-proof', `junit-${randomUUID()}.xml`);
  const configPath = join(directory, 'nextest.toml');
  await mkdir(join(targetDirectory, 'nextest', 'contract-proof'), { recursive: true });
  await writeFile(configPath, `[profile.contract-proof]\nretries = 0\n[profile.contract-proof.junit]\npath = "${basename(reportPath)}"\nstore-success-output = false\nstore-failure-output = false\nreport-skipped = "ignored"\n`);
  return {
    configPath,
    reportPath,
    async read(binaryId) {
      const details = await stat(reportPath).catch(() => { throw new Error('Missing fresh nextest JUnit report'); });
      if (details.size > MAX_REPORT_BYTES) throw new Error('Nextest JUnit report exceeds size limit');
      const xml = await readFile(reportPath, 'utf8').catch(() => { throw new Error('Missing fresh nextest JUnit report'); });
      return parseNextestJUnit(xml, binaryId);
    },
    async cleanup() {
      await rm(reportPath, { force: true });
      await rm(directory, { recursive: true, force: true });
    },
  };
}

function count(attributes, name) {
  const value = attributes[name];
  if (typeof value !== 'string' || !/^(0|[1-9][0-9]*)$/.test(value)) throw new Error(`Invalid JUnit ${name} count`);
  return Number(value);
}

export function parseNextestJUnit(xml, binaryId) {
  if (typeof xml !== 'string' || !xml || Buffer.byteLength(xml) > MAX_REPORT_BYTES) throw new Error('Missing or oversized nextest JUnit report');
  if (/<!DOCTYPE/i.test(xml)) throw new Error('JUnit DTD is forbidden');
  const parser = new SaxesParser();
  const stack = [];
  const passed = new Set();
  const ignored = new Set();
  let rootCounts;
  let suiteCounts;
  let suiteCount = 0;
  let cases = 0;
  let activeCase;
  parser.on('doctype', () => { throw new Error('JUnit DTD is forbidden'); });
  parser.on('error', (error) => { throw error; });
  parser.on('opentag', ({ name, attributes }) => {
    const parent = stack.at(-1);
    if (!parent) {
      if (name !== 'testsuites' || rootCounts) throw new Error('Unexpected JUnit document root');
      rootCounts = { tests: count(attributes, 'tests'), failures: count(attributes, 'failures'), errors: count(attributes, 'errors') };
    } else if (parent === 'testsuites') {
      if (name !== 'testsuite' || suiteCount++) throw new Error('JUnit must contain exactly one binary suite');
      if (attributes.name !== binaryId) throw new Error(`Unexpected JUnit binary: ${attributes.name}`);
      suiteCounts = { tests: count(attributes, 'tests'), failures: count(attributes, 'failures'), errors: count(attributes, 'errors') };
    } else if (parent === 'testsuite') {
      if (name !== 'testcase') throw new Error(`Unexpected JUnit suite child: ${name}`);
      if (attributes.classname !== binaryId || !/^[A-Za-z0-9_]+(?:::[A-Za-z0-9_]+)*$/.test(attributes.name ?? '')) throw new Error('Unexpected JUnit test identity');
      if (passed.has(attributes.name) || ignored.has(attributes.name)) throw new Error(`Duplicate JUnit test: ${attributes.name}`);
      activeCase = attributes.name;
      passed.add(attributes.name);
      cases += 1;
    } else if (parent === 'testcase') {
      if (name === 'skipped') {
        if (!passed.has(activeCase)) throw new Error('Duplicate JUnit skipped status');
        passed.delete(activeCase);
        ignored.add(activeCase);
      } else if (['failure', 'error', 'rerunFailure', 'rerunError', 'flakyFailure', 'flakyError'].includes(name)) {
        throw new Error(`JUnit case did not pass: ${name}`);
      } else throw new Error(`Unexpected JUnit case child: ${name}`);
    } else throw new Error(`Unexpected nested JUnit element: ${name}`);
    stack.push(name);
  });
  parser.on('closetag', () => {
    if (stack.pop() === 'testcase') activeCase = undefined;
  });
  parser.write(xml).close();
  if (!rootCounts || !suiteCounts || suiteCount !== 1 || stack.length || cases === 0) throw new Error('Incomplete or empty nextest JUnit report');
  if (rootCounts.failures || rootCounts.errors || suiteCounts.failures || suiteCounts.errors || rootCounts.tests !== cases || suiteCounts.tests !== cases) throw new Error('Nextest JUnit counts do not match executed cases');
  if (passed.size === 0) throw new Error('Nextest JUnit selected no executed tests');
  return { passed, ignored };
}
