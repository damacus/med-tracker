const fs = require('node:fs');

const optionNames = ['report-path', 'runs', 'perf-threshold', 'a11y-threshold', 'bp-threshold'];

function parseArguments(argumentsList) {
  const options = {};

  for (let index = 0; index < argumentsList.length; index += 2) {
    const option = argumentsList[index];
    const value = argumentsList[index + 1];

    if (!optionNames.includes(option?.slice(2)) || value === undefined || value.startsWith('--')) {
      throw new Error('Usage: node bin/lighthouse_score_gate.js --report-path PATH --runs COUNT --perf-threshold SCORE --a11y-threshold SCORE --bp-threshold SCORE');
    }

    options[option.slice(2)] = value;
  }

  if (optionNames.some((name) => options[name] === undefined)) {
    throw new Error('Usage: node bin/lighthouse_score_gate.js --report-path PATH --runs COUNT --perf-threshold SCORE --a11y-threshold SCORE --bp-threshold SCORE');
  }

  return options;
}

function numericOption(options, name, { integer = false } = {}) {
  const value = Number(options[name]);

  if (!Number.isFinite(value) || (integer && !Number.isInteger(value))) {
    throw new Error(`Invalid ${name}: ${options[name]}`);
  }

  return value;
}

function reportPath(reportPathPrefix, run) {
  return `${reportPathPrefix}-${run}.report.json`;
}

function isRecord(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function scoreFor(report, category, filePath) {
  const score = report.categories?.[category]?.score;

  if (!Number.isFinite(score) || score < 0 || score > 1) {
    throw new Error(`Malformed Lighthouse JSON report: ${filePath}`);
  }

  return score;
}

function loadReport(filePath) {
  if (!fs.existsSync(filePath)) {
    throw new Error(`Missing Lighthouse JSON report: ${filePath}`);
  }

  let report;

  try {
    report = JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    throw new Error(`Malformed Lighthouse JSON report: ${filePath}`);
  }

  if (!isRecord(report) || !isRecord(report.categories) || !isRecord(report.audits)) {
    throw new Error(`Malformed Lighthouse JSON report: ${filePath}`);
  }

  return {
    accessibility: scoreFor(report, 'accessibility', filePath),
    bestPractices: scoreFor(report, 'best-practices', filePath),
    filePath,
    performance: scoreFor(report, 'performance', filePath),
    report
  };
}

function scorePercent(score) {
  return Math.floor(score * 100);
}

function printScore(label, score, threshold) {
  console.log(`${label}:`.padEnd(16) + ` ${scorePercent(score)}% (threshold: ${threshold}%)`);
}

function failedAudits(report) {
  return Object.entries(report.audits)
    .filter(([, audit]) => isRecord(audit) && Number.isFinite(audit.score) && audit.score < 1)
    .map(([id, audit]) => ({ score: scorePercent(audit.score), title: audit.title || id }))
    .sort((left, right) => left.score - right.score)
    .slice(0, 10);
}

function run() {
  const options = parseArguments(process.argv.slice(2));
  const runs = numericOption(options, 'runs', { integer: true });
  const thresholds = {
    accessibility: numericOption(options, 'a11y-threshold'),
    bestPractices: numericOption(options, 'bp-threshold'),
    performance: numericOption(options, 'perf-threshold')
  };

  if (runs < 1 || runs % 2 === 0) {
    throw new Error(`runs must be a positive odd number: ${options.runs}`);
  }

  if (Object.values(thresholds).some((threshold) => threshold < 0 || threshold > 100)) {
    throw new Error('Thresholds must be between 0 and 100');
  }

  const reports = Array.from({ length: runs }, (_, index) => loadReport(reportPath(options['report-path'], index + 1)));
  const selected = [...reports].sort((left, right) => left.performance - right.performance)[Math.floor(reports.length / 2)];

  console.log(`Selected median performance report: ${selected.filePath}`);
  printScore('Performance', selected.performance, thresholds.performance);
  printScore('Accessibility', selected.accessibility, thresholds.accessibility);
  printScore('Best Practices', selected.bestPractices, thresholds.bestPractices);

  const failures = [
    ['Performance', selected.performance, thresholds.performance],
    ['Accessibility', selected.accessibility, thresholds.accessibility],
    ['Best Practices', selected.bestPractices, thresholds.bestPractices]
  ].filter(([, score, threshold]) => scorePercent(score) < threshold);

  if (failures.length === 0) {
    console.log('All scores meet thresholds.');
    return;
  }

  failures.forEach(([label]) => console.log(`${label} score below threshold!`));
  console.log('Top 10 failed audits:');
  failedAudits(selected.report).forEach((audit) => console.log(`  [${audit.score}%] ${audit.title}`));
  process.exitCode = 1;
}

try {
  run();
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
