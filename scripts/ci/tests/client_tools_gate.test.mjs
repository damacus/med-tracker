import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { classify } from '../classify.mjs';

const workflow = readFileSync(new URL('../../../.github/workflows/ci.yml', import.meta.url), 'utf8');
const clientJob = workflow.match(/\n  client_tools:\n(?<body>[\s\S]*?)(?=\n  rust_port:\n)/)?.groups?.body;
const mise = readFileSync(new URL('../../../mise.toml', import.meta.url), 'utf8');

test('client quality gate installs only its pinned tools and runs the Task CI contract', () => {
  assert.ok(clientJob, 'client_tools job was not found');
  assert.match(clientJob, /arduino\/setup-task@[a-f0-9]{40}/);
  assert.match(clientJob, /apt-get install[^\n]*\bfish\b/);
  assert.match(clientJob, /taiki-e\/install-action@1168f6810a3cb30a3514bf5756a3b7f0422c6df8/);
  for (const tool of ['cargo-nextest', 'cargo-machete', 'cargo-deny']) {
    const version = mise.match(new RegExp(`^"cargo:${tool}" = "([^"]+)"$`, 'm'))?.[1];
    assert.ok(version, `${tool} version missing from mise.toml`);
    assert.ok(clientJob.includes(`${tool}@${version}`), `${tool} installer drifted from mise.toml`);
  }
  assert.match(clientJob, /fallback: none/);
  assert.match(clientJob, /run: task client-tools:ci/);
  assert.doesNotMatch(clientJob, /run: cargo (fmt|check|clippy|test)|mise install/);
  assert.ok(clientJob.indexOf('apt-get install') < clientJob.indexOf('run: task client-tools:ci'));
  assert.ok(clientJob.indexOf('taiki-e/install-action') < clientJob.indexOf('run: task client-tools:ci'));
});

test('shared guardrail policy changes select both application and client checks', () => {
  for (const path of ['deny.toml', 'lefthook.yml']) {
    const { selected } = classify([path]);
    for (const suite of ['loco', 'rust', 'policy']) {
      assert.equal(selected[suite], true, `${path} missed ${suite}`);
    }
  }
  const { selected } = classify(['mise.toml']);
  for (const suite of ['loco', 'rust', 'policy', 'rails', 'rust_port']) {
    assert.equal(selected[suite], true, `mise.toml lost ${suite}`);
  }
});

test('installed skill metadata selects policy checks while skill Markdown keeps Markdown checks', () => {
  for (const path of ['.agents/skills/grill-me/agents/openai.yaml', 'skills-lock.json']) {
    const { selected } = classify([path]);
    assert.equal(selected.policy, true, `${path} missed policy checks`);
    assert.equal(selected.loco, false, `${path} selected Loco without an application change`);
    assert.equal(selected.rust, false, `${path} selected client Rust without a client change`);
  }
  const markdown = classify(['.agents/skills/grill-me/SKILL.md']).selected;
  assert.equal(markdown.markdown, true);
  assert.equal(markdown.policy, false);
});
