# Test Coverage & Mutation Testing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up mutation testing (mutant) for the first time, add an enforced coverage gate, and close the highest-value unit-test gaps — proving each new spec is meaningful by killing surviving mutants.

**Architecture:** Three moves. (1) Introduce `mutant` behind a spike that de-risks Ruby 4.0.4 compatibility and the OSS license. (2) Turn SimpleCov's existing branch coverage into an enforced ratchet (`minimum_coverage`) and add a non-blocking incremental mutation job to CI. (3) Write characterization + behaviour specs for currently-untested pure logic (services, serializers, policies, logic-bearing models) and the unguarded admin user-management edge cases, driving each to high mutation coverage. Every spec targets a class via its **constant** (`RSpec.describe SomeClass`) so mutant can auto-select covering specs.

**Tech Stack:** Ruby 4.0.4, Rails 8.1, RSpec, SimpleCov (branch coverage), FactoryBot + Rails fixtures, Pundit (+ pundit-matchers), shoulda-matchers, Docker via `task`, `mutant` + `mutant-rspec`.

---

## Current Status (measured 2026-06-09)

This is the assessment requested in the brief. Numbers are app files vs. matching `*_spec.rb`.

| Layer | App files | Have spec | **No spec** | Notes |
|---|---:|---:|---:|---|
| Models | 41 | ~24 | ~17 | ~6 are trivial Rodauth join models (≤8 lines) — out of scope. ~10 carry real logic. |
| Services | 101 | 23 | **~45** | Biggest gap. Entire `smart_insights` detector subsystem, `global_search` result queries, onboarding, `nhs_dmd` helpers, `ai_medication` untested. |
| Policies | 20 | 12 | **8** | `application_policy` (base), `dosage_policy`, `admin_dashboard_policy`, `admin_nhs_dmd_import_policy`, `app_settings_policy`, `location_membership_policy`, `notification_preference_policy`, `policy_helpers` concern. |
| Serializers | 8 | 1 | **7** | Only `MedicationSerializer` has a spec. |
| Presenters | 6 | 5 | 1 | `schedules/card_presenter`. |
| Jobs / Mailers | 7 | 5 | 2 | The 2 missing are framework base classes (trivial). |
| Domain | 4 | 4 | 0 | Fully covered. |
| Helpers | 1 | 0 | 1 | `application_helper`. |

**Tooling status**

- SimpleCov runs with **branch coverage on** (`.simplecov`) but **no `minimum_coverage` threshold** → coverage is reported, never enforced.
- CI (`.github/workflows/ci.yml`) uploads a coverage artifact on the `test_non_system` job (`COVERAGE: true`) but the build never fails on coverage.
- **No mutation testing exists** anywhere (confirmed: no `mutant`/`mutest` in `Gemfile`, `Gemfile.lock`, config, or CI).
- The repo is **public** (`damacus/med-tracker`) but has **no `LICENSE` file** — relevant because mutant's free tier requires an OSS licence.

**Specifically-requested "add/remove users" functionality:** the happy-path turbo flows for create/update/destroy/activate/verify ARE covered (`spec/requests/admin_create_update_turbo_spec.rb`, `spec/requests/admin_turbo_actions_spec.rb`). The **guard branches are not**: a grep for `cannot_deactivate_self`, `missing_account` returns **zero** spec hits. The "can't deactivate yourself", "verify with no account", "activate", and HTML (non-turbo) format paths in `Admin::UsersController` are untested. Phase 4 closes these.

## Decisions (locked)

1. **Engine: `mutant`** — actively maintained, richest mutators, has `--since <ref>` incremental mode. Cost: needs a `LICENSE` file + OSS licence token, and Ruby 4.0.4 support must be proven (Task 1.2 spike).
2. **Scope: focused initial pass** — infra + gate + the highest-value gaps (all 8 policies, all 7 serializers, pure-logic services, logic-bearing models, admin user edge cases). The long tail of remaining services/components is enumerated in [Appendix A](#appendix-a-prioritised-backlog-not-in-this-plan), not implemented here.
3. **CI: coverage gate + advisory mutation** — `minimum_coverage` **blocks** the build; the mutation job runs `--since origin/main` and is **non-blocking** (`continue-on-error: true`) until the signal is trusted.

---

## Execution Addendum — spike findings (2026-06-09)

The Phase 1 spike ran and **succeeded**. Several plan assumptions were corrected; these override the original text below where they conflict:

- **Engine works on Ruby 4.0.4.** `mutant 0.16.3` + `mutant-rspec 0.16.3` install and run. `mutant run` against `MedicationFriendlyName` selected the subject + its spec and executed 203 mutations.
- **Licensing is now trivial — no token, no gem.** Mutant 0.16 **removed the `mutant-license` gem entirely**. You declare `usage: opensource` (free for public OSS projects) in `config/mutant.yml`. There is **no** human-gated license step. (The MIT `LICENSE` from Task 1.1 is still correct hygiene and supports the opensource claim.)
- **Mutant belongs in the `:test` Gemfile group, NOT `:tools`.** The test bundle excludes `:tools` (that's why `rubocop`/`rubycritic` use a separate `tools-test` service). Mutant needs rspec + DB + app code, so it lives in `:test`.
- **Command conventions** (the original `task internal:run …` lines do not work — `internal:run` is `internal: true`):
  - Run a spec: `task test TEST_FILE=<spec_path>`
  - Run mutation on a subject: `task mutation SUBJECT='<Constant>'` (added to `Taskfile.yml`)
  - Arbitrary setup command in the test container: `task test:exec CMD='<cmd>'` (added to `Taskfiles/test.yml`)
  - Coverage env propagates through compose: prefix with `COVERAGE=true SIMPLECOV_COMMAND_NAME=… task test …` or use `task test:exec CMD='env COVERAGE=true … rspec --tag ~browser'`.
- **Parser caveat:** `parser` loads `parser/ruby33`, so files using 3.4+/4.0-only syntax may fail to mutate (warning only; conservative pure-logic targets are fine). Watch per-subject.
- **Equivalent mutants are expected.** 100% per subject is often unreachable without changing app code (e.g. `MedicationFriendlyName` tops out ~89%: `.squish` is redundant with `.split`, `|| name.blank?` is redundant with the `friendly.blank?` guard, and the regex decimal branch is dead because `.delete('.,')` runs first). Aim high, kill every *killable* mutant, and note equivalents in the commit message. Mutant's `ignore:` is method-level (too coarse to hide a single equivalent), so do not use it to fake 100%. The CI mutation job is advisory/non-blocking, so sub-100% per subject is acceptable.

## File Structure

**New files**

- `LICENSE` — MIT licence text (unblocks mutant OSS tier; standard hygiene for a public repo).
- `config/mutant.yml` — mutant integration/config (rspec integration, `app` includes, Rails env require).
- `docs/superpowers/plans/2026-06-09-test-coverage-and-mutation-testing.md` — this plan.
- New spec files (one per subject) under `spec/services`, `spec/serializers/api/v1`, `spec/policies`, `spec/models` — exact paths listed per task.

**Modified files**

- `Gemfile` — add `mutant`, `mutant-rspec`, `mutant-license` (`:tools` group, `require: false`).
- `.simplecov` — add enforced `minimum_coverage line:/branch:`.
- `Taskfile.yml` — add `mutation` and `mutation:since` tasks.
- `.github/workflows/ci.yml` — add non-blocking `mutation` job; ensure coverage gate fires on `test_non_system`.
- `TESTING.md` — document the mutation workflow and coverage gate.
- `spec/requests/admin_turbo_actions_spec.rb` — add the missing guard-branch examples.

**Responsibility boundaries:** each spec file owns exactly one subject class and `RSpec.describe`s it by constant. Infra files are config-only. No app/* runtime code changes except the two genuine bug-fix opportunities flagged inline in Phase 4 (only if a spec goes red).

---

## Subject Loop Protocol (named procedure — referenced by every Phase 3 task)

The app code already exists, so this is **mutation-driven characterization testing**, not classic red→green. For each subject the loop is:

1. **Write the spec** — `RSpec.describe TheConstant`. Cover every observable branch you can see in the source.
2. **Run rspec — expect GREEN.** The code exists; the spec should pass immediately.
   `task internal:run ENVIRONMENT=test COMMAND='bundle exec rspec <spec_path>'`
   - If it goes RED, you've found a real defect or a wrong assumption. STOP and use superpowers:systematic-debugging before continuing. Do **not** bend the spec to match a bug without confirming intent.
   - If a factory lacks an attribute the subject needs, add the attribute/trait to the factory — that's expected, not a placeholder.
3. **Run mutant on the subject — observe survivors.**
   `task test:up` (once per session) then `task mutation SUBJECT='<match-expression>'`
4. **Kill survivors.** Each surviving mutant = an assertion you're missing. Add focused examples until mutant reports `Mutations: N / Killed: N / Alive: 0` (100% for the subject). If a survivor is genuinely equivalent/not worth killing, record it in `config/mutant.yml` `ignore:` with a one-line comment — never leave it silently alive.
5. **Commit** — spec (+ any factory tweak) in one commit: `git commit -m "test: cover <Subject> (100% mutation)"`.

**Match-expression syntax** (mutant): a bare constant (`MedicationFriendlyName`) mutates the class and all its methods; `'GlobalSearch::ResultBuilder*'` (quote the `*` for the shell) mutates the namespace. Use the value given in each task's "Mutant subject" line.

> All `task internal:run` / `task mutation` commands run inside the Docker `web-test` container (see `Taskfiles/internal.yml`). Per project rules, **always use `task`, never raw `docker compose`.** The non-Docker fallbacks from `CLAUDE.md` (`bundle exec rspec …`, `bundle exec mutant run …`) are equivalent if you have a local Postgres + `DATABASE_URL`.

---

## Phase 1 — Mutation testing infrastructure (spike-first)

### Task 1.1: Add an MIT LICENSE

**Files:**
- Create: `LICENSE`

- [ ] **Step 1: Create the licence file**

```text
MIT License

Copyright (c) 2026 Dan Webb

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

> Confirm "Dan Webb" / year is how the owner wants to be attributed before committing. If unsure, ask.

- [ ] **Step 2: Commit**

```bash
git add LICENSE
git commit -m "chore: add MIT LICENSE"
```

---

### Task 1.2: Add mutant gems + obtain OSS licence — **THE SPIKE** (de-risk Ruby 4.0.4)

This task is a go/no-go gate. If mutant cannot load the app on Ruby 4.0.4 after a reasonable effort, STOP and report back rather than forcing it (see fallback at the end).

**Files:**
- Modify: `Gemfile` (`:tools` group)

- [ ] **Step 1: Add the gems**

In `Gemfile`, inside the existing `group :tools do` block (where `brakeman`, `rubycritic` live), add:

```ruby
  # Mutation testing [https://github.com/mbj/mutant]
  gem 'mutant', require: false
  gem 'mutant-license', require: false
  gem 'mutant-rspec', require: false
```

- [ ] **Step 2: Wire the OSS licence**

Follow the current mutant licensing instructions: <https://github.com/mbj/mutant#licensing>. mutant is free for open-source projects; the `mutant-license` gem is fetched from mbj's private gem source using an OSS token tied to the project. Apply the exact `source:`/token mechanism the README specifies **at the time of execution** (it has changed across versions — do not hard-code a stale URL). The repo now has a `LICENSE` (Task 1.1), satisfying the "freely licensed" requirement.

If CI will run mutation (Phase 5), the token must be available at `bundle install` time. Note whichever mechanism you used here — Phase 5 Task 5.1 wires the same secret into CI.

- [ ] **Step 3: Install**

```bash
task internal:run ENVIRONMENT=test COMMAND='bundle install'
```

Expected: bundle resolves with `mutant`, `mutant-rspec`, `mutant-license`. If bundler errors on Ruby 4.0.4 (native ext / `unparser` / `parser` incompatibility), record the exact error — that is the core spike finding.

- [ ] **Step 4: Verify mutant loads and sees the integration**

```bash
task internal:run ENVIRONMENT=test COMMAND='env RAILS_ENV=test bundle exec mutant environment show'
```

Expected: mutant prints its environment (integration, includes, subjects) with **no licence error and no parser crash**. Defer the `config/mutant.yml` details to Task 1.3 — at this point you only need mutant to start.

- [ ] **Step 5: Commit (only if Steps 3–4 succeed)**

```bash
git add Gemfile Gemfile.lock
git commit -m "build: add mutant for mutation testing"
```

**Fallback if mutant cannot run on Ruby 4.0.4:** revert this commit, and report the blocker with the captured error. Options to raise with the owner: (a) pin a mutant/parser version that supports 4.0.x, (b) run mutation in a Ruby 3.4 sidecar container against the same specs, or (c) fall back to `mutest`. Do **not** proceed to Phase 3's mutation steps until the engine runs; Phases 3–4 specs are still valuable on their own (the coverage gate in Phase 2 still applies), so you may continue writing specs and skip only the `task mutation` steps, marking them blocked.

---

### Task 1.3: Add mutant config

**Files:**
- Create: `config/mutant.yml`

- [ ] **Step 1: Write the config**

```yaml
---
# Mutant configuration. See: https://github.com/mbj/mutant
# Run via: task mutation SUBJECT=SomeClass
integration:
  name: rspec
includes:
  - app
requires:
  - ./config/environment
# Subjects we deliberately do not mutate yet (add with a reason, never leave a
# survivor silently alive — see the Subject Loop Protocol).
# Example:
# ignore:
#   - 'SomeClass#some_equivalent_method'
```

> `requires: ['./config/environment']` boots Rails so mutant can resolve `app/` constants. If the spike (1.2) showed Rails boot is too slow/fragile inside mutant's workers, narrow `requires` to the specific files per subject instead, and document that here. Validate with the next step.

- [ ] **Step 2: Verify the config is picked up**

```bash
task test:up
task internal:run ENVIRONMENT=test COMMAND='env RAILS_ENV=test bundle exec mutant environment show'
```

Expected: output lists `integration: rspec`, `includes: app`, and a non-zero subject count. Iterate on `config/mutant.yml` until this is true.

- [ ] **Step 3: Commit**

```bash
git add config/mutant.yml
git commit -m "build: add mutant configuration"
```

---

### Task 1.4: Add `task mutation` runners

**Files:**
- Modify: `Taskfile.yml` (under top-level `tasks:`, alongside `test` / `rubocop`)

- [ ] **Step 1: Add the tasks**

```yaml
  mutation:
    desc: Run mutation testing on a subject (SUBJECT=ClassName)
    summary: |
      Run mutant against a subject match expression inside the test container.
      Requires the test database — run `task test:up` first.
      Usage: task mutation SUBJECT=MedicationFriendlyName
      Usage: task mutation SUBJECT='GlobalSearch::ResultBuilder*'
    cmds:
      - task: internal:run
        vars:
          ENVIRONMENT: test
          COMMAND: 'env RAILS_ENV=test bundle exec mutant run -- "{{ .SUBJECT }}"'

  mutation:since:
    desc: Mutation-test only subjects changed since a git ref (REF, default origin/main)
    summary: |
      Incremental mutation run — mutates only subjects touched since REF.
      Usage: task mutation:since
      Usage: task mutation:since REF=HEAD~3
    vars:
      ref: '{{ .REF | default "origin/main" }}'
    cmds:
      - task: internal:run
        vars:
          ENVIRONMENT: test
          COMMAND: 'env RAILS_ENV=test bundle exec mutant run --since {{ .ref }}'
```

- [ ] **Step 2: Verify the task is wired (no subject = mutant usage/empty result, not a task error)**

```bash
task --list | grep mutation
```

Expected: `mutation` and `mutation:since` appear with their descriptions.

- [ ] **Step 3: Commit**

```bash
git add Taskfile.yml
git commit -m "build: add task mutation runners"
```

---

### Task 1.5: Prove the loop end-to-end on one subject

This validates the whole toolchain before you rely on it. `MedicationFriendlyName` is pure and already has a spec (`spec/serializers/...`? no — it has no spec). It DOES have logic worth mutating. Use it as the canary.

**Files:**
- Create: `spec/services/medication_friendly_name_spec.rb`

Source under test (`app/services/medication_friendly_name.rb`): `MedicationFriendlyName.derive(name:, code:)` returns a trimmed "friendly" prefix of `name`, or `nil` when `code`/`name` is blank, when the result is blank, or when the result equals `name`. Words stop at dosage/stop-words.

- [ ] **Step 1: Write the spec**

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationFriendlyName do
  describe '.derive' do
    it 'returns the leading non-dosage words as the friendly name' do
      result = described_class.derive(name: 'Paracetamol 500mg tablets', code: '12345')
      expect(result).to eq('Paracetamol')
    end

    it 'strips parenthetical manufacturer text before deriving' do
      result = described_class.derive(name: 'Movicol Paediatric (Norgine) oral powder', code: '12345')
      expect(result).to eq('Movicol Paediatric')
    end

    it 'returns nil when the code is blank' do
      expect(described_class.derive(name: 'Paracetamol 500mg tablets', code: '')).to be_nil
    end

    it 'returns nil when the name is blank' do
      expect(described_class.derive(name: '', code: '12345')).to be_nil
    end

    it 'returns nil when the friendly name would equal the original name' do
      expect(described_class.derive(name: 'Paracetamol', code: '12345')).to be_nil
    end

    it 'returns nil when no leading word survives the stop-word filter' do
      expect(described_class.derive(name: 'tablets 500mg', code: '12345')).to be_nil
    end

    it 'treats percentage and unit-suffixed numbers as dosage tokens' do
      expect(described_class.derive(name: 'Hydrocortisone 1% cream', code: '12345')).to eq('Hydrocortisone')
    end
  end
end
```

- [ ] **Step 2: Run rspec — expect GREEN**

```bash
task internal:run ENVIRONMENT=test COMMAND='bundle exec rspec spec/services/medication_friendly_name_spec.rb'
```

Expected: 7 examples, 0 failures.

- [ ] **Step 3: Run mutant on the subject**

```bash
task test:up
task mutation SUBJECT=MedicationFriendlyName
```

Expected: mutant boots, mutates `MedicationFriendlyName`, and reports a coverage summary. Some mutants may survive — that is the point of the next step. **Success criterion for this task is that the pipeline runs and produces a coverage figure**, proving the engine works.

- [ ] **Step 4: Kill survivors to reach `Alive: 0`**

Add examples for each survivor mutant reports (e.g. the `delete_suffix(',')`, the `== name` guard, each regex branch). Re-run Step 3 until `Alive: 0`.

- [ ] **Step 5: Commit**

```bash
git add spec/services/medication_friendly_name_spec.rb
git commit -m "test: cover MedicationFriendlyName (100% mutation)"
```

✅ **Checkpoint:** mutation testing now works locally. The rest of Phase 3 reuses the Subject Loop Protocol.

---

## Phase 2 — Coverage gate

### Task 2.1: Measure the current baseline

**Files:** none (measurement only)

- [ ] **Step 1: Run the non-system suite with coverage**

```bash
task test:up
task internal:run ENVIRONMENT=test COMMAND='env COVERAGE=true SIMPLECOV_COMMAND_NAME=baseline bundle exec rspec --tag ~browser'
```

- [ ] **Step 2: Read the measured line & branch coverage**

```bash
task internal:run ENVIRONMENT=test COMMAND='cat coverage/.last_run.json'
```

Expected: JSON like `{"result":{"line":91.2,"branch":83.4}}`. **Record both numbers.**

---

### Task 2.2: Enforce a coverage ratchet in `.simplecov`

**Files:**
- Modify: `.simplecov`

- [ ] **Step 1: Add `minimum_coverage` set to the measured baseline, rounded DOWN to a whole percent**

Worked example: if Task 2.1 reported `line: 91.2, branch: 83.4`, set `line: 91, branch: 83`. Insert immediately after `enable_coverage :branch` inside the `SimpleCov.start 'rails' do` block:

```ruby
  enable_coverage :branch

  # Enforced ratchet. Raise these as coverage improves; never lower without a
  # recorded reason. Values are the measured baseline (docs plan 2026-06-09),
  # rounded down to whole percents.
  minimum_coverage line: 91, branch: 83
```

(Substitute YOUR measured, rounded-down numbers.)

- [ ] **Step 2: Verify the gate passes at baseline**

```bash
task internal:run ENVIRONMENT=test COMMAND='env COVERAGE=true bundle exec rspec --tag ~browser'
```

Expected: suite passes AND SimpleCov prints no "Coverage … is below the expected minimum" line (exit status 0).

- [ ] **Step 3: Verify the gate actually fails when under threshold (prove it bites)**

Temporarily bump the threshold above baseline, e.g. `minimum_coverage line: 100, branch: 100`, rerun Step 2's command.
Expected: SimpleCov prints `Line coverage (… %) is below the expected minimum coverage (100.00%).` and the process exits non-zero. Then **revert** to the baseline numbers.

- [ ] **Step 4: Commit**

```bash
git add .simplecov
git commit -m "test: enforce SimpleCov coverage ratchet"
```

> CI already sets `COVERAGE: true` on `test_non_system`, so this gate now blocks PRs via that job — no workflow change needed for the gate itself. (The advisory mutation job is added in Phase 5.) After Phases 3–4 land, run Task 2.1 again and raise the thresholds to the new baseline in a final commit.

---

## Phase 3 — Close high-value unit gaps (mutation-driven)

Every task below follows the **Subject Loop Protocol**. Each gives the spec path, full starting spec, and the exact `Mutant subject` to pass as `SUBJECT=`. After GREEN + `Alive: 0`, commit per the protocol.

### 3A — Pure-logic services

#### Task 3A.1: `MedicationPlanClassifier`

**Files:** Create `spec/services/medication_plan_classifier_spec.rb` · **Mutant subject:** `MedicationPlanClassifier`

Source (`app/services/medication_plan_classifier.rb`): `direct?` is true for supplement categories (`vitamin`/`supplement`/`mineral`, case-insensitive) or `schedule_type == 'prn'`; `administration_kind` is `'routine'` for supplements else `'as_needed'`; `schedule_type` falls back through the passed value → `medication.default_schedule_type` → `'multiple_daily'`.

- [ ] **Step 1: Write the spec**

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationPlanClassifier do
  let(:medication) { instance_double(Medication, category: category, default_schedule_type: default_type) }
  let(:category) { 'Analgesic' }
  let(:default_type) { nil }

  describe '#direct?' do
    it 'is true for a supplement category regardless of case' do
      expect(described_class.new(medication: instance_double(Medication, category: 'Vitamin', default_schedule_type: nil)).direct?).to be(true)
    end

    it 'is true when the schedule_type is prn' do
      expect(described_class.new(medication: medication, schedule_type: 'prn').direct?).to be(true)
    end

    it 'is false for a non-supplement with a non-prn schedule_type' do
      expect(described_class.new(medication: medication, schedule_type: 'multiple_daily').direct?).to be(false)
    end
  end

  describe '#administration_kind' do
    it "is 'routine' for supplement categories" do
      classifier = described_class.new(medication: instance_double(Medication, category: 'mineral', default_schedule_type: nil))
      expect(classifier.administration_kind).to eq('routine')
    end

    it "is 'as_needed' otherwise" do
      expect(described_class.new(medication: medication).administration_kind).to eq('as_needed')
    end
  end

  describe '#schedule_type' do
    it 'prefers the explicitly passed schedule_type' do
      expect(described_class.new(medication: medication, schedule_type: 'daily').schedule_type).to eq('daily')
    end

    it 'falls back to the medication default when none is passed' do
      classifier = described_class.new(medication: instance_double(Medication, category: 'x', default_schedule_type: 'weekly'))
      expect(classifier.schedule_type).to eq('weekly')
    end

    it "falls back to 'multiple_daily' when nothing is set" do
      expect(described_class.new(medication: medication).schedule_type).to eq('multiple_daily')
    end
  end
end
```

- [ ] **Steps 2–5:** Run rspec (GREEN) → `task mutation SUBJECT=MedicationPlanClassifier` → kill survivors (watch the `||` in `direct?` and the `.presence` chain in `schedule_type`) → commit `test: cover MedicationPlanClassifier (100% mutation)`.

#### Task 3A.2: `GlobalSearch::ResultBuilder` (+ `GlobalSearch::Result`)

**Files:** Create `spec/services/global_search/result_builder_spec.rb` · **Mutant subject:** `'GlobalSearch::ResultBuilder*'`

Source (`app/services/global_search/result_builder.rb`): scoring is exact — `100` exact match, `80` prefix, `60` substring, `40` a secondary value contains the query, else `0`. All comparisons are on `strip.downcase`. This is a prime mutation target (every boundary number matters).

- [ ] **Step 1: Write the spec**

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe GlobalSearch::ResultBuilder do
  subject(:builder) { described_class.new(query: '  Para ') }

  def score_for(title, secondary: [])
    builder.build(type: :medication, title: title, subtitle: 's', path: '/p', secondary_values: secondary).score
  end

  it 'scores an exact (case/space-insensitive) title 100' do
    expect(score_for('para')).to eq(100)
    expect(score_for('PARA')).to eq(100)
  end

  it 'scores a prefix match 80' do
    expect(score_for('Paracetamol')).to eq(80)
  end

  it 'scores a substring (non-prefix) match 60' do
    expect(score_for('Co-paracetamol')).to eq(60)
  end

  it 'scores a secondary-value match 40 when the title does not match' do
    expect(score_for('Ibuprofen', secondary: ['Para brand'])).to eq(40)
  end

  it 'scores 0 when nothing matches' do
    expect(score_for('Ibuprofen', secondary: ['Nurofen'])).to eq(0)
  end

  it 'builds a Result carrying the supplied fields' do
    result = builder.build(type: :person, title: 'Para', subtitle: 'sub', path: '/people/1')
    expect(result).to have_attributes(type: :person, title: 'Para', subtitle: 'sub', path: '/people/1', score: 100)
  end

  describe '#rescore' do
    it 'recomputes score from secondary values while preserving identity fields' do
      original = builder.build(type: :medication, title: 'Ibuprofen', subtitle: 'sub', path: '/m/1')
      rescored = builder.rescore(original, secondary_values: ['Para'])
      expect(rescored).to have_attributes(title: 'Ibuprofen', path: '/m/1', score: 40)
    end
  end

  describe GlobalSearch::Result do
    it 'serialises to the public JSON shape' do
      json = described_class.new(type: :medication, title: 'Para', subtitle: 's', path: '/p', score: 100).as_json
      expect(json).to eq(type: :medication, title: 'Para', subtitle: 's', path: '/p', score: 100)
    end
  end
end
```

- [ ] **Steps 2–5:** rspec GREEN → `task mutation SUBJECT='GlobalSearch::ResultBuilder*'` → kill survivors (the four threshold numbers + `start_with?` vs `include?` are the key mutants) → commit. Then separately `task mutation SUBJECT='GlobalSearch::Result'` and ensure `Alive: 0` (the `as_json` spec above covers it).

#### Task 3A.3: `MedicationParamsNormalizer`

**Files:** Create `spec/services/medication_params_normalizer_spec.rb` · **Mutant subject:** `MedicationParamsNormalizer`

Source (`app/services/medication_params_normalizer.rb`): mutates a permitted-params hash. (a) `default_schedule_config`: permits config keys when it responds to `permit`, else `to_h`, else `JSON.parse`, else `{}` on blank/parse error. (b) `dosage_records_attributes`: for `default_for_adults`/`default_for_children`, keeps only the **last** selected non-destroyed record as `'1'`, forcing earlier ones to `'0'`.

- [ ] **Step 1: Write the spec**

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationParamsNormalizer do
  let(:config_keys) { %i[times frequency] }

  def normalize(permitted)
    described_class.call(permitted, schedule_config_keys: config_keys)
  end

  describe 'default_schedule_config normalisation' do
    it 'leaves params untouched when the key is absent' do
      expect(normalize({ name: 'x' })).to eq(name: 'x')
    end

    it 'coerces a blank config to an empty hash' do
      expect(normalize({ default_schedule_config: '' })[:default_schedule_config]).to eq({})
    end

    it 'parses a JSON string config' do
      result = normalize({ default_schedule_config: '{"frequency":"daily"}' })
      expect(result[:default_schedule_config]).to eq('frequency' => 'daily')
    end

    it 'returns an empty hash for invalid JSON' do
      expect(normalize({ default_schedule_config: 'not json' })[:default_schedule_config]).to eq({})
    end
  end

  describe 'dosage default de-duplication' do
    it 'keeps only the last selected adult-default record' do
      records = {
        '0' => { default_for_adults: '1' },
        '1' => { default_for_adults: '1' }
      }
      normalize({ dosage_records_attributes: records })
      expect(records['0'][:default_for_adults]).to eq('0')
      expect(records['1'][:default_for_adults]).to eq('1')
    end

    it 'ignores records marked for destruction when choosing the survivor' do
      records = {
        '0' => { default_for_children: '1' },
        '1' => { default_for_children: '1', _destroy: '1' }
      }
      normalize({ dosage_records_attributes: records })
      expect(records['0'][:default_for_children]).to eq('1')
    end

    it 'does nothing when there are no dosage records' do
      expect(normalize({ name: 'x' })).to eq(name: 'x')
    end
  end
end
```

- [ ] **Steps 2–5:** rspec GREEN → `task mutation SUBJECT=MedicationParamsNormalizer` → kill survivors (the `[0...-1]` slice and the `reject`/`select` predicates are the interesting mutants) → commit.

### 3B — `smart_insights` detector subsystem

All detectors subclass `SmartInsights::Detectors::Base` and consume a `SmartInsights::Context`. Unit-test each with `instance_double(SmartInsights::Context, …)` (verified doubles are on, and every stubbed method is a real `Context` method). Assert the **branch logic and structural insight fields** (`key`/`family`/`severity`) — these are the mutation-critical parts. I18n strings come from real locale files; assert presence, not exact copy.

#### Task 3B.1: `SmartInsights::Insight` & `SmartInsights::Result` (Data types)

**Files:** Create `spec/services/smart_insights/data_types_spec.rb` · **Mutant subjects:** `'SmartInsights::Insight'`, `'SmartInsights::Result'`

These are `Data.define` value objects. A tiny spec pins their members so refactors that drop/rename a field are caught.

- [ ] **Step 1: Write the spec**

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SmartInsights::Insight do
  it 'exposes all insight members' do
    insight = described_class.new(
      key: :k, family: :f, severity: :info, title: 't', summary: 's',
      detail: 'd', metric_label: 'ml', metric_value: 'mv', cta_path: nil
    )
    expect(insight).to have_attributes(key: :k, family: :f, severity: :info, metric_value: 'mv', cta_path: nil)
  end
end

RSpec.describe SmartInsights::Result do
  it 'exposes all result members' do
    result = described_class.new(primary_insight: nil, insights: [], learning_state?: true, evidence_summary: 'x')
    expect(result).to have_attributes(insights: [], learning_state?: true, evidence_summary: 'x')
  end
end
```

- [ ] **Steps 2–5:** rspec GREEN → `task mutation SUBJECT='SmartInsights::Insight'` then `SUBJECT='SmartInsights::Result'` → these may report no mutations (acceptable — document if so) → commit `test: pin SmartInsights value objects`.

#### Task 3B.2: `SmartInsights::Detectors::AdherenceStreak` (exemplar — boundary killing)

**Files:** Create `spec/services/smart_insights/detectors/adherence_streak_spec.rb` · **Mutant subject:** `'SmartInsights::Detectors::AdherenceStreak'`

Source: counts a trailing run of days where `expected.positive? && actual >= expected`; emits one `:adherence_streak`/`:positive` insight only when the streak `>= 3`.

- [ ] **Step 1: Write the spec**

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SmartInsights::Detectors::AdherenceStreak do
  def context_with(daily_data)
    instance_double(SmartInsights::Context, daily_data: daily_data)
  end

  def day(expected:, actual:)
    { expected: expected, actual: actual }
  end

  it 'returns no insights for a trailing streak of 2 (below the threshold of 3)' do
    data = [day(expected: 1, actual: 0), day(expected: 1, actual: 1), day(expected: 1, actual: 1)]
    expect(described_class.new(context_with(data)).call).to eq([])
  end

  it 'emits a positive adherence insight for a trailing streak of exactly 3' do
    data = [day(expected: 1, actual: 1), day(expected: 1, actual: 1), day(expected: 1, actual: 1)]
    insights = described_class.new(context_with(data)).call
    expect(insights.size).to eq(1)
    expect(insights.first).to have_attributes(key: :adherence_streak, family: :adherence, severity: :positive)
  end

  it 'counts a day where actual exactly equals expected as adherent' do
    data = Array.new(3) { day(expected: 2, actual: 2) }
    expect(described_class.new(context_with(data)).call.size).to eq(1)
  end

  it 'breaks the streak on a day with zero expected doses' do
    data = [day(expected: 0, actual: 0), day(expected: 1, actual: 1), day(expected: 1, actual: 1)]
    expect(described_class.new(context_with(data)).call).to eq([])
  end

  it 'breaks the streak when actual is below expected' do
    data = [day(expected: 2, actual: 1), day(expected: 1, actual: 1), day(expected: 1, actual: 1)]
    expect(described_class.new(context_with(data)).call).to eq([])
  end
end
```

- [ ] **Steps 2–5:** rspec GREEN → `task mutation SUBJECT='SmartInsights::Detectors::AdherenceStreak'` → kill survivors (`< 3` boundary, `>=` vs `>`, `.positive?`) → commit.

#### Task 3B.3: `SmartInsights::Detectors::MissedDosePattern`

**Files:** Create `spec/services/smart_insights/detectors/missed_dose_pattern_spec.rb` · **Mutant subject:** `'SmartInsights::Detectors::MissedDosePattern'`

Source: longest run of `missed_day?` (`expected.positive? && actual < expected`); emits `:missed_dose_pattern`/`:warning` when the longest streak `>= 2`.

- [ ] **Step 1: Write the spec**

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SmartInsights::Detectors::MissedDosePattern do
  def context_with(daily_data)
    instance_double(SmartInsights::Context, daily_data: daily_data)
  end

  def day(expected:, actual:)
    { expected: expected, actual: actual }
  end

  it 'stays silent when the longest missed streak is 1' do
    data = [day(expected: 1, actual: 0), day(expected: 1, actual: 1), day(expected: 1, actual: 0)]
    expect(described_class.new(context_with(data)).call).to eq([])
  end

  it 'warns when two consecutive days are missed' do
    data = [day(expected: 1, actual: 0), day(expected: 1, actual: 0), day(expected: 1, actual: 1)]
    insights = described_class.new(context_with(data)).call
    expect(insights.first).to have_attributes(key: :missed_dose_pattern, family: :adherence, severity: :warning)
  end

  it 'does not count a day with zero expected as missed' do
    data = [day(expected: 0, actual: 0), day(expected: 0, actual: 0)]
    expect(described_class.new(context_with(data)).call).to eq([])
  end

  it 'tracks the longest streak, not the most recent' do
    data = [day(expected: 1, actual: 0), day(expected: 1, actual: 0), day(expected: 1, actual: 1), day(expected: 1, actual: 0)]
    expect(described_class.new(context_with(data)).call.size).to eq(1)
  end
end
```

- [ ] **Steps 2–5:** rspec GREEN → mutant → kill survivors (the `each_with_object([0,0])` reducer, `< 2` threshold, `[longest, current].max`) → commit.

#### Task 3B.4: `SmartInsights::Detectors::PrnUsage`

**Files:** Create `spec/services/smart_insights/detectors/prn_usage_spec.rb` · **Mutant subject:** `'SmartInsights::Detectors::PrnUsage'`

Source: emits `:prn_usage`/`:info` only when `context.prn_sources.any? && context.prn_takes.any?`.

- [ ] **Step 1: Write the spec**

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SmartInsights::Detectors::PrnUsage do
  def context_with(prn_sources:, prn_takes:)
    instance_double(SmartInsights::Context, prn_sources: prn_sources, prn_takes: prn_takes)
  end

  it 'is silent with no PRN sources' do
    expect(described_class.new(context_with(prn_sources: [], prn_takes: [double])).call).to eq([])
  end

  it 'is silent with PRN sources but no takes' do
    expect(described_class.new(context_with(prn_sources: [double], prn_takes: [])).call).to eq([])
  end

  it 'emits an info insight when there are PRN sources and takes' do
    insights = described_class.new(context_with(prn_sources: [double], prn_takes: [double, double])).call
    expect(insights.first).to have_attributes(key: :prn_usage, family: :as_needed, severity: :info)
  end
end
```

- [ ] **Steps 2–5:** rspec GREEN → mutant → kill survivors (`&&` → both `||` and each operand) → commit.

#### Task 3B.5: `SmartInsights::Detectors::InventoryRisk`

**Files:** Create `spec/services/smart_insights/detectors/inventory_risk_spec.rb` · **Mutant subject:** `'SmartInsights::Detectors::InventoryRisk'`

Source: takes the first `context.inventory_alerts`; severity `:urgent` when `alert[:low_stock]` else `:warning`; summary uses a "zero" variant when `days_left <= 0`.

- [ ] **Step 1: Write the spec**

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SmartInsights::Detectors::InventoryRisk do
  def context_with(alerts)
    instance_double(SmartInsights::Context, inventory_alerts: alerts)
  end

  def alert(low_stock:, days_left:, name: 'Paracetamol')
    { low_stock: low_stock, days_left: days_left, medication_name: name }
  end

  it 'is silent with no inventory alerts' do
    expect(described_class.new(context_with([])).call).to eq([])
  end

  it 'is urgent when the first alert is low stock' do
    insights = described_class.new(context_with([alert(low_stock: true, days_left: 1)])).call
    expect(insights.first).to have_attributes(key: :inventory_risk, family: :inventory, severity: :urgent)
  end

  it 'is a warning when the first alert is not low stock' do
    insights = described_class.new(context_with([alert(low_stock: false, days_left: 5)])).call
    expect(insights.first.severity).to eq(:warning)
  end

  it 'uses the zero-days summary when days_left is not positive' do
    insight = described_class.new(context_with([alert(low_stock: true, days_left: 0)])).call.first
    zero_summary = I18n.t('smart_insights.detectors.inventory_risk.summary_zero', medication_name: 'Paracetamol')
    expect(insight.summary).to eq(zero_summary)
  end

  it 'uses the countdown summary when days_left is positive' do
    insight = described_class.new(context_with([alert(low_stock: false, days_left: 3)])).call.first
    expected = I18n.t('smart_insights.detectors.inventory_risk.summary', medication_name: 'Paracetamol', count: 3)
    expect(insight.summary).to eq(expected)
  end
end
```

- [ ] **Steps 2–5:** rspec GREEN → mutant → kill survivors (the ternary, `<= 0` boundary, `.first`) → commit.

#### Task 3B.6: `SmartInsights::Detectors::ScheduleHygiene`

**Files:** Create `spec/services/smart_insights/detectors/schedule_hygiene_spec.rb` · **Mutant subject:** `'SmartInsights::Detectors::ScheduleHygiene'`

Source: from `context.active_schedules`, find the first that is `schedule_type_multiple_daily?` AND has no configured `times`; emit `:schedule_hygiene`/`:info`.

- [ ] **Step 1: Write the spec**

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SmartInsights::Detectors::ScheduleHygiene do
  def schedule(multiple_daily:, times:, name: 'Paracetamol')
    instance_double(
      Schedule,
      schedule_type_multiple_daily?: multiple_daily,
      schedule_config: { 'times' => times },
      medication_name: name
    )
  end

  def context_with(active_schedules)
    instance_double(SmartInsights::Context, active_schedules: active_schedules)
  end

  it 'is silent when there are no active schedules' do
    expect(described_class.new(context_with([])).call).to eq([])
  end

  it 'is silent when a multiple-daily schedule has configured times' do
    expect(described_class.new(context_with([schedule(multiple_daily: true, times: ['08:00'])])).call).to eq([])
  end

  it 'is silent for a non-multiple-daily schedule even without times' do
    expect(described_class.new(context_with([schedule(multiple_daily: false, times: [])])).call).to eq([])
  end

  it 'flags a multiple-daily schedule with blank times' do
    insights = described_class.new(context_with([schedule(multiple_daily: true, times: ['', nil])])).call
    expect(insights.first).to have_attributes(key: :schedule_hygiene, family: :schedule, severity: :info)
  end
end
```

- [ ] **Steps 2–5:** rspec GREEN → mutant → kill survivors (the `&&`, `compact_blank.empty?`) → commit.

> **Task 3B.7 (`TimingConsistency`)** is more involved (real time arithmetic, `IndexQuery::MINIMUM_EVENTS`, schedule occurrence expansion). Write it **after** 3B.2–3B.6 are green. Cover: below-evidence returns `[]`; `on_time_ratio < 0.8` returns `[]`; `>= 0.8` emits `:timing_consistency`/`:positive`; a take within `WINDOW_MINUTES` (90) counts and one outside does not. Use `instance_double(SmartInsights::Context, …)` plus lightweight `Schedule`/take doubles. If the occurrence-expansion proves too coupled to mock cleanly, mark it for the Appendix A backlog rather than writing a brittle spec — note that decision in the commit message.

### 3C — Serializers

All serializers are plain objects exposing `as_json`. Pattern: build the record (factory where one exists; fixtures for the user/account chain), call `described_class.new(record).as_json`, assert the full hash. Asserting every key/value is what kills the mutants (each dropped/renamed key is a live mutant otherwise).

#### Task 3C.1: `Api::V1::LocationSerializer`

**Files:** Create `spec/serializers/api/v1/location_serializer_spec.rb` · **Mutant subject:** `'Api::V1::LocationSerializer'`

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::V1::LocationSerializer do
  it 'serialises identity, description and an ISO8601 updated_at' do
    location = create(:location, name: 'Kitchen', description: 'Top shelf')
    json = described_class.new(location).as_json
    expect(json).to eq(
      id: location.id,
      name: 'Kitchen',
      description: 'Top shelf',
      updated_at: location.updated_at.iso8601
    )
  end
end
```

- [ ] Run rspec GREEN → `task mutation SUBJECT='Api::V1::LocationSerializer'` → kill survivors → commit.

#### Task 3C.2: `Api::V1::NotificationPreferenceSerializer`

**Files:** Create `spec/serializers/api/v1/notification_preference_serializer_spec.rb` · **Mutant subject:** `'Api::V1::NotificationPreferenceSerializer'`

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::V1::NotificationPreferenceSerializer do
  it 'serialises the preference with HH:MM:SS period times' do
    preference = create(:notification_preference, enabled: true, morning_time: '08:30', night_time: nil)
    json = described_class.new(preference).as_json
    expect(json).to include(
      id: preference.id,
      person_id: preference.person_id,
      enabled: true,
      updated_at: preference.updated_at.iso8601,
      morning_time: '08:30:00',
      night_time: nil
    )
  end
end
```

- [ ] Run rspec GREEN → mutant (`SUBJECT='Api::V1::NotificationPreferenceSerializer'`) → kill survivors (the `strftime` format string, the `&.`) → commit. (Adjust the `:notification_preference` factory if it lacks `morning_time`/`night_time` columns — check `spec/factories/notification_preferences.rb` first.)

#### Task 3C.3: `Api::V1::ScheduleSerializer`

**Files:** Create `spec/serializers/api/v1/schedule_serializer_spec.rb` · **Mutant subject:** `'Api::V1::ScheduleSerializer'`

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::V1::ScheduleSerializer do
  it 'serialises association, timing and dosing data' do
    schedule = create(:schedule)
    json = described_class.new(schedule).as_json
    expect(json).to include(
      id: schedule.id,
      person_id: schedule.person_id,
      medication_id: schedule.medication_id,
      frequency: schedule.frequency,
      dose_cycle: schedule.dose_cycle,
      active: schedule.active?,
      max_daily_doses: schedule.max_daily_doses,
      min_hours_between_doses: schedule.min_hours_between_doses
    )
    expect(json[:start_date]).to eq(schedule.start_date&.iso8601)
    expect(json[:end_date]).to eq(schedule.end_date&.iso8601)
    expect(json[:updated_at]).to eq(schedule.updated_at.iso8601)
  end
end
```

- [ ] Run rspec GREEN → mutant (`SUBJECT='Api::V1::ScheduleSerializer'`) → kill survivors → commit.

#### Task 3C.4: `Api::V1::PersonMedicationSerializer`

**Files:** Create `spec/serializers/api/v1/person_medication_serializer_spec.rb` · **Mutant subject:** `'Api::V1::PersonMedicationSerializer'`

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::V1::PersonMedicationSerializer do
  it 'serialises association, schedule and dosing-limit data' do
    pm = create(:person_medication)
    json = described_class.new(pm).as_json
    expect(json).to include(
      id: pm.id,
      person_id: pm.person_id,
      medication_id: pm.medication_id,
      dose_amount: pm.dose_amount&.to_f,
      dose_unit: pm.dose_unit,
      dose_cycle: pm.dose_cycle,
      administration_kind: pm.administration_kind,
      notes: pm.notes,
      position: pm.position,
      max_daily_doses: pm.max_daily_doses,
      min_hours_between_doses: pm.min_hours_between_doses,
      updated_at: pm.updated_at.iso8601
    )
  end
end
```

- [ ] Run rspec GREEN → mutant (`SUBJECT='Api::V1::PersonMedicationSerializer'`) → kill survivors (`&.to_f`) → commit.

#### Task 3C.5: `Api::V1::MedicationTakeSerializer`

**Files:** Create `spec/serializers/api/v1/medication_take_serializer_spec.rb` · **Mutant subject:** `'Api::V1::MedicationTakeSerializer'`

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::V1::MedicationTakeSerializer do
  it 'serialises source, event and subject data' do
    take = create(:medication_take)
    json = described_class.new(take).as_json
    expect(json).to include(
      id: take.id,
      client_uuid: take.client_uuid,
      schedule_id: take.schedule_id,
      person_medication_id: take.person_medication_id,
      taken_from_medication_id: take.taken_from_medication_id,
      taken_from_location_id: take.taken_from_location_id,
      dose_amount: take.dose_amount&.to_f,
      dose_unit: take.dose_unit,
      taken_at: take.taken_at&.iso8601,
      updated_at: take.updated_at.iso8601,
      person_id: take.person&.id,
      medication_id: take.medication&.id
    )
  end
end
```

- [ ] Run rspec GREEN → mutant (`SUBJECT='Api::V1::MedicationTakeSerializer'`) → kill survivors → commit. (Inspect `spec/factories/medication_takes.rb` to confirm it builds `person`/`medication` via its source; add a trait if needed.)

#### Task 3C.6: `Api::V1::PersonSerializer`

**Files:** Create `spec/serializers/api/v1/person_serializer_spec.rb` · **Mutant subject:** `'Api::V1::PersonSerializer'`

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::V1::PersonSerializer do
  it 'serialises identity plus age, location ids and notification preference id' do
    person = create(:person, name: 'Alex')
    location = create(:location)
    person.locations << location
    preference = create(:notification_preference, person: person)

    json = described_class.new(person).as_json
    expect(json).to include(
      id: person.id,
      name: 'Alex',
      email: person.email,
      person_type: person.person_type,
      has_capacity: person.has_capacity,
      updated_at: person.updated_at.iso8601,
      age: person.age,
      location_ids: [location.id],
      notification_preference_id: preference.id
    )
    expect(json[:date_of_birth]).to eq(person.date_of_birth&.iso8601)
  end
end
```

- [ ] Run rspec GREEN → mutant (`SUBJECT='Api::V1::PersonSerializer'`) → kill survivors (`&.iso8601`, `&.id`) → commit. (Confirm `Person#email` / `#age` / `#locations` / `#notification_preference` exist; they're used by the serializer so they must.)

#### Task 3C.7: `Api::V1::MeSerializer`

**Files:** Create `spec/serializers/api/v1/me_serializer_spec.rb` · **Mutant subject:** `'Api::V1::MeSerializer'`

This one needs the full user→person→account chain; use fixtures (`fixtures :all`, `users(:admin)`) as the policy specs do, since there is no user/account factory.

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::V1::MeSerializer do
  fixtures :all

  let(:user) { users(:admin) }

  it 'serialises the user, nested person and account' do
    json = described_class.new(user).as_json
    expect(json).to include(
      id: user.id,
      email_address: user.email_address,
      role: user.role,
      active: user.active
    )
    expect(json[:person]).to eq(Api::V1::PersonSerializer.new(user.person).as_json)
    expect(json[:account]).to include(
      id: user.person.account.id,
      email: user.person.account.email,
      status: user.person.account.status
    )
  end
end
```

- [ ] Run rspec GREEN → mutant (`SUBJECT='Api::V1::MeSerializer'`) → kill survivors → commit. (If `users(:admin)` has no linked account in fixtures, pick a fixture user that does, or add the account fixture.)

### 3D — Policies

Follow the existing policy-spec house style (`spec/policies/medication_policy_spec.rb`): `type: :policy`, `fixtures :all`, role-based `users(:admin|doctor|nurse|carer|parent)`. Assert each permission per role; that kills the `administrator?`/`||`/`&&` mutants.

#### Task 3D.1: `ApplicationPolicy` (base defaults + Scope contract)

**Files:** Create `spec/policies/application_policy_spec.rb` · **Mutant subject:** `'ApplicationPolicy*'`

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe ApplicationPolicy, type: :policy do
  fixtures :all

  subject(:policy) { described_class.new(users(:admin), :record) }

  it 'denies every default action' do
    aggregate_failures do
      expect(policy.index?).to be(false)
      expect(policy.show?).to be(false)
      expect(policy.create?).to be(false)
      expect(policy.new?).to be(false)
      expect(policy.update?).to be(false)
      expect(policy.edit?).to be(false)
      expect(policy.destroy?).to be(false)
    end
  end

  it 'aliases new? to create? and edit? to update?' do
    expect(policy.method(:new?).original_name).to eq(:create?)
    expect(policy.method(:edit?).original_name).to eq(:update?)
  end

  describe ApplicationPolicy::Scope do
    it 'requires subclasses to implement #resolve' do
      expect { described_class.new(users(:admin), User.all).resolve }.to raise_error(NoMethodError, /resolve/)
    end
  end
end
```

- [ ] Run rspec GREEN → `task mutation SUBJECT='ApplicationPolicy*'` → kill survivors → commit. (The private `carer_with_patient?`/`parent_with_dependent_patient?` helpers are exercised through concrete subclasses; if mutant flags them as uncovered here, note them and rely on subclass policy specs — or add a focused example via an anonymous subclass that implements `person_id_for_authorization`.)

#### Task 3D.2: `PolicyHelpers` concern

**Files:** Create `spec/policies/policy_helpers_spec.rb` · **Mutant subject:** `'PolicyHelpers'`

Test through a throwaway host policy so the private predicates are reachable.

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe PolicyHelpers, type: :policy do
  fixtures :all

  let(:host_class) do
    Class.new(ApplicationPolicy) do
      def admin_public? = admin?
      def admin_or_clinician_public? = admin_or_clinician?
      def medical_staff_public? = medical_staff?
      def carer_or_parent_public? = carer_or_parent?
    end
  end

  def policy_for(user) = host_class.new(user, :record)

  it 'admin? is true only for administrators' do
    expect(policy_for(users(:admin)).admin_public?).to be(true)
    expect(policy_for(users(:doctor)).admin_public?).to be(false)
  end

  it 'admin_or_clinician? covers admin, doctor and nurse' do
    aggregate_failures do
      expect(policy_for(users(:admin)).admin_or_clinician_public?).to be(true)
      expect(policy_for(users(:doctor)).admin_or_clinician_public?).to be(true)
      expect(policy_for(users(:nurse)).admin_or_clinician_public?).to be(true)
      expect(policy_for(users(:carer)).admin_or_clinician_public?).to be(false)
    end
  end

  it 'medical_staff? covers doctor and nurse only' do
    aggregate_failures do
      expect(policy_for(users(:doctor)).medical_staff_public?).to be(true)
      expect(policy_for(users(:nurse)).medical_staff_public?).to be(true)
      expect(policy_for(users(:admin)).medical_staff_public?).to be(false)
    end
  end

  it 'carer_or_parent? covers carer and parent only' do
    aggregate_failures do
      expect(policy_for(users(:carer)).carer_or_parent_public?).to be(true)
      expect(policy_for(users(:parent)).carer_or_parent_public?).to be(true)
      expect(policy_for(users(:admin)).carer_or_parent_public?).to be(false)
    end
  end

  it 'returns false (not nil) when there is no user' do
    expect(policy_for(nil).admin_public?).to be(false)
  end
end
```

- [ ] Run rspec GREEN → mutant (`SUBJECT='PolicyHelpers'`) → kill survivors (each `||` operand, the `|| false` tail) → commit.

#### Task 3D.3: Admin-only policies (`AdminDashboard`, `AdminNhsDmdImport`, `AppSettings`, `LocationMembership`)

**Files:** Create four spec files · **Mutant subjects:** `'AdminDashboardPolicy*'`, `AdminNhsDmdImportPolicy`, `'AppSettingsPolicy*'`, `LocationMembershipPolicy`

These four share a shape: an admin-gated permission set (`LocationMembership` uses the `admin?` helper; the others check `administrator?` directly). Spec template — repeat per policy, swapping the class, the permission methods, and (for the two with a `Scope`) the scope assertion:

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AdminDashboardPolicy, type: :policy do
  fixtures :all

  it 'permits index? only for administrators' do
    expect(described_class.new(users(:admin), :dashboard).index?).to be(true)
    expect(described_class.new(users(:doctor), :dashboard).index?).to be(false)
    expect(described_class.new(nil, :dashboard).index?).to be(false)
  end

  describe AdminDashboardPolicy::Scope do
    it 'returns the given scope unchanged' do
      scope = User.all
      expect(described_class.new(users(:admin), scope).resolve).to eq(scope)
    end
  end
end
```

Concrete per-file assertions:
- `spec/policies/admin_dashboard_policy_spec.rb` → `index?` + `Scope#resolve` (as above).
- `spec/policies/admin_nhs_dmd_import_policy_spec.rb` → `new?` and `create?` both admin-gated; **no Scope** (omit that block).
- `spec/policies/app_settings_policy_spec.rb` → `show?` and `update?` admin-gated; `Scope#resolve` returns the scope.
- `spec/policies/location_membership_policy_spec.rb` → `create?` and `destroy?` admin-gated (via `admin?` helper → admins true, doctors/nil false); **no Scope**.

- [ ] For each: write spec → rspec GREEN → `task mutation SUBJECT='<Policy>'` → kill survivors → commit `test: cover <Policy> (100% mutation)`.

#### Task 3D.4: `DosagePolicy` (delegates to `MedicationPolicy`)

**Files:** Create `spec/policies/dosage_policy_spec.rb` · **Mutant subject:** `DosagePolicy`

Source delegates `show?` to medication policy, and ties `create?`/`new?`/`update?`/`edit?`/`destroy?` to the medication policy's `update?`/`update?`. Verify the delegation maps correctly per role.

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe DosagePolicy, type: :policy do
  fixtures :all

  let(:dosage) { dosages(:paracetamol_adult) } # any dosage whose .medication resolves
  subject(:policy) { described_class.new(user, dosage) }

  context 'as a doctor (can update medications, cannot destroy)' do
    let(:user) { users(:doctor) }

    it 'mirrors MedicationPolicy#update? for write actions and allows destroy via update?' do
      aggregate_failures do
        expect(policy.show?).to be(true)
        expect(policy.create?).to be(true)
        expect(policy.new?).to be(true)
        expect(policy.update?).to be(true)
        expect(policy.edit?).to be(true)
        expect(policy.destroy?).to be(true) # destroy? delegates to medication update?, not destroy?
      end
    end
  end

  context 'as a nurse (read-only on medications)' do
    let(:user) { users(:nurse) }

    it 'denies create/update/destroy' do
      aggregate_failures do
        expect(policy.create?).to be(false)
        expect(policy.update?).to be(false)
        expect(policy.destroy?).to be(false)
        expect(policy.show?).to be(true)
      end
    end
  end
end
```

- [ ] Write spec → rspec GREEN (confirm a `dosages(...)` fixture exists whose `.medication` is set; if not, build one) → `task mutation SUBJECT=DosagePolicy` → kill survivors → commit.

#### Task 3D.5: `NotificationPreferencePolicy` (+ Scope)

**Files:** Create `spec/policies/notification_preference_policy_spec.rb` · **Mutant subject:** `'NotificationPreferencePolicy*'`

Source: `show?` true for admin/clinician OR when the record belongs to the user's person; `Scope` returns none (no user), all (clinician), own (otherwise).

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe NotificationPreferencePolicy, type: :policy do
  fixtures :all

  describe '#show?' do
    it 'permits a clinician to view any preference' do
      preference = create(:notification_preference)
      expect(described_class.new(users(:doctor), preference).show?).to be(true)
    end

    it 'permits a user to view their own person preference' do
      user = users(:carer)
      preference = create(:notification_preference, person: user.person)
      expect(described_class.new(user, preference).show?).to be(true)
    end

    it "denies a user viewing another person's preference" do
      user = users(:carer)
      other = create(:notification_preference)
      expect(described_class.new(user, other).show?).to be(false)
    end

    it 'denies when the user has no person' do
      preference = create(:notification_preference)
      user_without_person = User.new(role: :carer) # unsaved → #person is nil
      expect(described_class.new(user_without_person, preference).show?).to be(false)
    end
  end

  describe NotificationPreferencePolicy::Scope do
    it 'returns all for a clinician' do
      create(:notification_preference)
      scope = described_class.new(users(:doctor), NotificationPreference.all).resolve
      expect(scope).to eq(NotificationPreference.all)
    end

    it 'returns only the user-person rows otherwise' do
      user = users(:carer)
      own = create(:notification_preference, person: user.person)
      create(:notification_preference)
      expect(described_class.new(user, NotificationPreference.all).resolve).to contain_exactly(own)
    end
  end
end
```

- [ ] Write spec → rspec GREEN → `task mutation SUBJECT='NotificationPreferencePolicy*'` → kill survivors (the `&&`/`||`, `person_id ==`, the three Scope branches) → commit. (`users(:carer)` has a person; the unsaved `User.new` covers the no-person guard. If `create(:notification_preference, person: user.person)` conflicts with a uniqueness constraint on existing fixture data, build the preference against a fresh `create(:person)` and set that person on the user instead.)

### 3E — Logic-bearing models

#### Task 3E.1: `BarcodeCatalogEntry` (validations + `.normalize_gtin`)

**Files:** Create `spec/models/barcode_catalog_entry_spec.rb` · **Mutant subject:** `BarcodeCatalogEntry`

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe BarcodeCatalogEntry do
  describe '.normalize_gtin' do
    it 'strips all non-digit characters' do
      expect(described_class.normalize_gtin(' 5-012345_678900 ')).to eq('5012345678900')
    end

    it 'returns an empty string for nil' do
      expect(described_class.normalize_gtin(nil)).to eq('')
    end
  end

  describe 'validations' do
    subject { described_class.new(gtin: '5012345678900', display: 'X', source: 'curated') }

    it { is_expected.to validate_presence_of(:gtin) }
    it { is_expected.to validate_presence_of(:display) }
    it { is_expected.to validate_presence_of(:source) }
    it { is_expected.to validate_uniqueness_of(:gtin).scoped_to(:source) }
  end
end
```

- [ ] Write spec → rspec GREEN → `task mutation SUBJECT=BarcodeCatalogEntry` → kill survivors (the `\D` regex, `to_s`) → commit.

#### Task 3E.2: `NhsDmdBarcode` (validations + normalize + cache expiry)

**Files:** Create `spec/models/nhs_dmd_barcode_spec.rb` · **Mutant subject:** `NhsDmdBarcode`

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe NhsDmdBarcode do
  describe '.normalize_gtin' do
    it 'keeps only digits' do
      expect(described_class.normalize_gtin('  05012-345 ')).to eq('05012345')
    end
  end

  describe 'validations' do
    subject { described_class.new(gtin: '5012345678900', code: 'C', display: 'D', system: 'S') }

    it { is_expected.to validate_presence_of(:gtin) }
    it { is_expected.to validate_uniqueness_of(:gtin) }
    it { is_expected.to validate_presence_of(:code) }
    it { is_expected.to validate_presence_of(:display) }
    it { is_expected.to validate_presence_of(:system) }
  end

  it 'expires the barcode-lookup cache after commit' do
    record = described_class.new(gtin: '5012345678900', code: 'C', display: 'D', system: 'S')
    expect(NhsDmd::BarcodeLookup).to receive(:expire).with('5012345678900')
    record.save!
  end
end
```

- [ ] Write spec → rspec GREEN → `task mutation SUBJECT=NhsDmdBarcode` → kill survivors → commit.

#### Task 3E.3: `NativeDeviceToken` (validations + platform inclusion)

**Files:** Create `spec/models/native_device_token_spec.rb` · **Mutant subject:** `NativeDeviceToken`

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe NativeDeviceToken do
  subject { described_class.new(platform: 'ios') }

  it { is_expected.to belong_to(:account) }
  it { is_expected.to validate_presence_of(:device_token) }
  it { is_expected.to validate_uniqueness_of(:device_token) }
  it { is_expected.to validate_inclusion_of(:platform).in_array(%w[ios android]) }

  it 'freezes the platform allow-list' do
    expect(described_class::PLATFORMS).to eq(%w[ios android]).and be_frozen
  end
end
```

- [ ] Write spec → rspec GREEN → `task mutation SUBJECT=NativeDeviceToken` → kill survivors → commit. (`validate_uniqueness_of(:device_token)` needs a persisted record — provide an `account` via fixtures/factory if shoulda complains.)

#### Task 3E.4: `InvitationDependent` (custom validation)

**Files:** Create `spec/models/invitation_dependent_spec.rb` · **Mutant subject:** `InvitationDependent`

Source: valid only when `dependent.person_type` ∈ {minor, dependent_adult} AND `dependent.has_capacity == false`; uniqueness of `dependent_id` scoped to `invitation_id`.

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe InvitationDependent do
  let(:invitation) { create(:invitation) }

  it 'is valid for a minor without capacity' do
    dependent = create(:person, :minor)
    record = described_class.new(invitation: invitation, dependent: dependent)
    expect(record).to be_valid
  end

  it 'is valid for a dependent adult without capacity' do
    dependent = create(:person, :dependent_adult)
    expect(described_class.new(invitation: invitation, dependent: dependent)).to be_valid
  end

  it 'is invalid for an adult with capacity' do
    dependent = create(:person) # default adult, has_capacity true
    record = described_class.new(invitation: invitation, dependent: dependent)
    expect(record).not_to be_valid
    expect(record.errors[:dependent]).to be_present
  end

  it 'is invalid for a minor that somehow has capacity' do
    dependent = create(:person, :minor, has_capacity: true)
    expect(described_class.new(invitation: invitation, dependent: dependent)).not_to be_valid
  end

  it 'enforces uniqueness of dependent within an invitation' do
    dependent = create(:person, :minor)
    described_class.create!(invitation: invitation, dependent: dependent)
    dup = described_class.new(invitation: invitation, dependent: dependent)
    expect(dup).not_to be_valid
  end
end
```

- [ ] Write spec → rspec GREEN → `task mutation SUBJECT=InvitationDependent` → kill survivors (the `&&`, the `== false`, the `.in?` set) → commit.

#### Task 3E.5: `MedicationAssignment` (attribute typing)

**Files:** Create `spec/models/medication_assignment_spec.rb` · **Mutant subject:** `MedicationAssignment`

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationAssignment do
  it 'casts attributes to their declared types' do
    assignment = described_class.new(
      medication_id: '5', source_dosage_option_id: '7', dose_amount: '2.5', dose_unit: 'ml'
    )
    expect(assignment).to have_attributes(
      medication_id: 5,
      source_dosage_option_id: 7,
      dose_amount: BigDecimal('2.5'),
      dose_unit: 'ml'
    )
  end
end
```

- [ ] Write spec → rspec GREEN → `task mutation SUBJECT=MedicationAssignment` → kill survivors → commit. (Likely few/zero mutations — this pins the attribute contract. Document if mutant reports no subjects.)

#### Task 3E.6: `MedicationDailyConsumption`

**Files:** Create `spec/models/medication_daily_consumption_spec.rb` · **Mutant subject:** `MedicationDailyConsumption`

Source sums `schedule_rate + person_medication_rate`. Each active schedule contributes `(max_daily_doses / (cycle_period / 1.day)) * consumption_for(...)`; schedules/PMs with blank `max_daily_doses` contribute `0.0`. This touches several collaborators — prefer fixtures or factories that produce a `Medication` with one active schedule and one person_medication, and assert the numeric total.

- [ ] **Step 1: Write the spec** (shape — fill concrete numbers after inspecting `MedicationStockConsumption.quantity_for`):

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationDailyConsumption do
  it 'returns 0.0 when there are no active schedules or person medications' do
    medication = create(:medication)
    expect(described_class.new(medication).call).to eq(0.0)
  end

  it 'ignores schedules with a blank max_daily_doses' do
    medication = create(:medication)
    create(:schedule, medication: medication, max_daily_doses: nil)
    expect(described_class.new(medication).call).to eq(0.0)
  end

  it 'sums the daily consumption across active schedules' do
    medication = create(:medication)
    create(:schedule, medication: medication, max_daily_doses: 2, dose_cycle: 'daily')
    result = described_class.new(medication).call
    expect(result).to be > 0.0
  end
end
```

- [ ] **Step 2:** rspec GREEN. If the third example's exact value matters for mutation (it will — mutant mutates `+`, `*`, `/`), inspect `app/models/medication_stock_consumption.rb#quantity_for` and assert the **exact** total so mutant can distinguish `+`↔`-` and `*`↔`/`.
- [ ] **Steps 3–5:** `task mutation SUBJECT=MedicationDailyConsumption` → kill survivors (the arithmetic operators, the `next 0.0` guards, `cycle_period / 1.day`) → commit. If isolating exact numbers proves too coupled, downgrade to Appendix A and note why.

#### Task 3E.7: `MedicationTakeStockSource` & `MedicationTakeStockMutation`

**Files:** Create `spec/models/medication_take_stock_source_spec.rb` and `spec/models/medication_take_stock_mutation_spec.rb` · **Mutant subjects:** `MedicationTakeStockSource`, `MedicationTakeStockMutation`

These coordinate inventory/dosage resolution and are the most collaborator-heavy in 3E. Drive them with `instance_double`s for `take`, `inventory`, and `InventoryDosageOptionResolver` to keep the unit tests fast and focused on the branch logic (`tracked?`, `selected_dose?`, `in_stock?` guards; `decrement` returning `nil` vs a `StockChange`).

- [ ] **Step 1:** Write `MedicationTakeStockSource` spec covering: `tracked?` (inventory present AND (`current_supply` present OR `dosage_option` present)); `selected_dose?` true when inventory blank / not tracked-dosage, else dosage_option presence; `in_stock?` each early-return branch. Stub `InventoryDosageOptionResolver` via `allow(InventoryDosageOptionResolver).to receive(:new).and_return(resolver_double)`.
- [ ] **Step 2:** Write `MedicationTakeStockMutation` spec: `decrement` returns `nil` when `stock_source.tracked?` is false, else a `StockChange` with the decremented row; `inventory_in_stock?`/`inventory_matches_selected_dose?` delegate correctly. Inject a `decrementer` double via the constructor kwarg.
- [ ] **Steps 3–5:** rspec GREEN → mutant each subject → kill survivors → commit. If the collaborator graph makes a clean unit spec impractical, write a smaller integration-style spec with real factories instead, still `RSpec.describe` the constant, and note the choice.

> `InventoryDosageOptionResolver` and `TimingRestrictions` (concern) are valuable but heavily collaborator-coupled. Specs for them are in **Appendix A** (do them in a follow-up once 3A–3E are green), unless time allows — if you do them here, mock `inventory.dosage_records` and the `source` duck-type per the source in `app/services/inventory_dosage_option_resolver.rb`.

---

## Phase 4 — Admin user-management edge cases (the explicitly-requested gaps)

These are real, currently-untested guard branches in `app/controllers/admin/users_controller.rb`. Add them to the existing request spec so they live beside the happy-path turbo examples.

### Task 4.1: Cover the destroy/activate/verify guard branches

**Files:**
- Modify: `spec/requests/admin_turbo_actions_spec.rb`

- [ ] **Step 1: Read the existing file** to match its login/setup helpers (it already logs in as an admin and uses `users(:jane)`).

- [ ] **Step 2: Add the missing examples** (append inside the existing top-level `describe`):

```ruby
  describe 'DELETE /admin/users/:id when targeting yourself' do
    it 'refuses to deactivate the current admin and returns unprocessable content' do
      admin = users(:admin) # the logged-in user (match however the file logs in)

      delete admin_user_path(admin), headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).to include('target="flash"')
      expect(admin.reload).to be_active
    end

    it 'redirects with an alert for an HTML request' do
      admin = users(:admin)

      delete admin_user_path(admin)

      expect(response).to redirect_to(admin_users_path)
      follow_redirect!
      expect(admin.reload).to be_active
    end
  end

  describe 'POST /admin/users/:id/verify when the user has no account' do
    it 'returns unprocessable content and a missing-account alert' do
      user = users(:jane)
      user.person.update!(account: nil) # detach the account; adjust if the association differs

      post verify_admin_user_path(user), headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).to include('target="flash"')
    end
  end

  describe 'POST /admin/users/:id/activate (HTML)' do
    it 'reactivates a deactivated user and redirects with a notice' do
      user = users(:jane)
      user.deactivate!

      post activate_admin_user_path(user)

      expect(response).to redirect_to(admin_users_path)
      expect(user.reload).to be_active
    end
  end
```

- [ ] **Step 3: Run the spec**

```bash
task internal:run ENVIRONMENT=test COMMAND='bundle exec rspec spec/requests/admin_turbo_actions_spec.rb'
```

Expected: all examples (existing + 4 new) pass. If "targeting yourself" fails because the file logs in as a different fixture user, use **that** user as the self-target. If detaching the account violates a DB constraint, instead create a user whose person has no account, or stub `@user.person.account` to `nil`.

- [ ] **Step 4: (optional) Confirm the controller branches are now covered**

```bash
task test:up
task mutation SUBJECT='Admin::UsersController#destroy'
task mutation SUBJECT='Admin::UsersController#verify'
```

Expected: the `== current_user` guard and the `unless account` guard report killed mutants. (Controller mutation is slower; this step is advisory — skip if Phase 1 marked mutation blocked.)

- [ ] **Step 5: Commit**

```bash
git add spec/requests/admin_turbo_actions_spec.rb
git commit -m "test: cover admin user deactivate-self, verify-no-account, activate guards"
```

---

## Phase 5 — CI advisory mutation job, docs, and ratchet bump

### Task 5.1: Add a non-blocking incremental mutation job to CI

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Add the job** (sibling of `test_non_system`; copy its `services:`/`env:` block for Postgres):

```yaml
  mutation:
    runs-on: [ubuntu-latest]
    name: Mutation Testing (advisory, changed subjects)
    timeout-minutes: 20
    continue-on-error: true # advisory until the signal is trusted
    services:
      postgres:
        image: postgres:18-alpine
        env:
          POSTGRES_USER: ${{ secrets.POSTGRES_USER }}
          POSTGRES_PASSWORD: ${{ secrets.POSTGRES_PASSWORD }}
          POSTGRES_DB: medtracker_test
        ports: ['5432:5432']
        options: >-
          --health-cmd pg_isready --health-interval 10s
          --health-timeout 5s --health-retries 5
    env:
      TEST_DATABASE_ADAPTER: postgresql
      DATABASE_URL: postgres://${{ secrets.POSTGRES_USER }}:${{ secrets.POSTGRES_PASSWORD }}@localhost:5432/medtracker_test
      RAILS_ENV: test
    steps:
      - name: Checkout code
        uses: actions/checkout@df4cb1c069e1874edd31b4311f1884172cec0e10 # v6
        with:
          fetch-depth: 0 # mutant --since needs full history

      - name: Set up Ruby
        uses: ruby/setup-ruby@v1
        with:
          ruby-version: "4.0.4"
          bundler-cache: true

      - name: Set up database
        run: bundle exec rails db:migrate

      - name: Run mutation testing on changed subjects
        run: bundle exec mutant run --since origin/main
```

> The mutant OSS licence token (from Task 1.2) must be available to `bundle install` here. Wire it the same way you did locally — if it's a gem-source token, add it via `bundle config set` using a repo secret (e.g. `MUTANT_LICENSE_TOKEN`) in a step before `bundler-cache`, or commit it if mbj's OSS flow treats it as public. Resolve this from the Task 1.2 finding; do not invent a mechanism.

- [ ] **Step 2: Validate the workflow YAML**

```bash
task internal:run ENVIRONMENT=test COMMAND='ruby -ryaml -e "YAML.load_file(%q{.github/workflows/ci.yml}); puts %q{ci.yml OK}"'
```

Expected: `ci.yml OK`.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add advisory incremental mutation job"
```

### Task 5.2: Document the workflow in TESTING.md

**Files:**
- Modify: `TESTING.md`

- [ ] **Step 1: Append a "Mutation testing" + "Coverage gate" section** documenting: `task mutation SUBJECT=…`, `task mutation:since`, the `RSpec.describe <Constant>` requirement for mutant subject selection, the `config/mutant.yml` `ignore:` convention, and that `.simplecov`'s `minimum_coverage` blocks CI via the `test_non_system` job while the `mutation` CI job is advisory. Keep it consistent with the file's existing tone/format.

- [ ] **Step 2: Commit**

```bash
git add TESTING.md
git commit -m "docs: document mutation testing and coverage gate"
```

### Task 5.3: Re-baseline and raise the coverage ratchet

**Files:**
- Modify: `.simplecov`

- [ ] **Step 1: Re-measure** (Phases 3–4 added many specs):

```bash
task internal:run ENVIRONMENT=test COMMAND='env COVERAGE=true SIMPLECOV_COMMAND_NAME=rebaseline bundle exec rspec --tag ~browser'
task internal:run ENVIRONMENT=test COMMAND='cat coverage/.last_run.json'
```

- [ ] **Step 2: Raise `minimum_coverage`** in `.simplecov` to the new measured values, rounded down. Verify the suite still passes the gate (re-run Step 1's rspec command, expect exit 0).

- [ ] **Step 3: Commit**

```bash
git add .simplecov
git commit -m "test: raise coverage ratchet to new baseline"
```

---

## Self-Review

**Spec coverage of the brief:**
- "Areas with no tests / functionality not tested" → Current Status table + Phase 3 (services, serializers, policies, models) + Appendix A backlog. ✅
- "Removing/adding users specifically" → verified happy paths exist; Phase 4 closes the untested guard branches (deactivate-self, verify-no-account, activate, HTML formats). ✅
- "Full coverage of anything unit-testable" → focused pass covers all 8 policies + all 7 serializers + pure-logic services + logic-bearing models; remainder enumerated in Appendix A (decision #2). ✅
- "Initial pass: current status + how to improve" → Current Status section + phased plan. ✅
- "We have no mutation testing — implement that" → Phase 1 (mutant install + spike + config + task), Phase 5 (CI). ✅ Headline ask.
- "Unit playwright tests" → the existing Playwright/system suite is broad; no system gaps block this unit-focused pass. Any future Playwright gaps go to Appendix A. Noted, not silently dropped.

**Placeholder scan:** no "TBD/implement later". The two deliberately-deferred subjects (`TimingConsistency`, `InventoryDosageOptionResolver`/`TimingRestrictions`) are explicitly routed to Appendix A with reasons, not faked as complete. The coverage-threshold "fill in your measured number" steps include exact commands to obtain the number + a worked example — concrete, not hand-waving.

**Type/name consistency:** mutant subjects use real constants verified against source (`GlobalSearch::ResultBuilder`, `SmartInsights::Detectors::AdherenceStreak`, `Api::V1::PersonSerializer`, etc.). `task mutation` / `task mutation:since` names are consistent between Task 1.4, the protocol, and Phase 5. `minimum_coverage line:/branch:` keys match SimpleCov's API and the `enable_coverage :branch` already in `.simplecov`.

**Known risks called out in-plan:** (1) mutant on Ruby 4.0.4 — gated by the Task 1.2 spike with a defined fallback; (2) Rails boot inside mutant workers — validated in Task 1.3; (3) factory attribute gaps — handled by the protocol's GREEN step; (4) collaborator-heavy models — explicit downgrade-to-backlog escape hatches.

---

## Appendix A — Prioritised backlog (NOT in this plan)

Tackle in follow-up plans once the infra + focused pass land. Ordered by value/effort.

1. **`smart_insights` remainder:** `TimingConsistency` detector, `Context`, `IndexQuery`, `insight`-assembly result. (Pure-ish, high value.)
2. **`global_search` result queries:** `locations_results_query`, `medications_results_query`, `people_results_query`, `person_medications_results_query`, `schedules_results_query`, `record_results_query`, `global_search_commands_query`. (DB-coupled; integration specs.)
3. **Medication onboarding:** `medication_onboarding_create_service`, `_builder`, `_plan_builder`. (Core flows; ~200 lines each.)
4. **Inventory resolution:** `InventoryDosageOptionResolver`, `medication_stock_match_resolver`, `medication_stock_source_resolver`, `adjust_medication_inventory_service`, `medication_inventory_matcher`. (Complex; high regression value.)
5. **`nhs_dmd` import internals:** `release_archive_extractor`, `release_import_counts`, `release_import_progress`, `vmp_resolver`. + `nhs_website_content/client`.
6. **`ai_medication`:** `ruby_llm_assistant`, `source_page(_client)`, `suggestion`, tools (`extract_medication_guidance`, `search_medication_sources`). (WebMock-stubbed.)
7. **Model concerns:** `TimingRestrictions`, `OtelInstrumented`.
8. **Remaining models:** `medication_params_normalizer` adjacents, `open_food_facts`/`open_products_facts` `result_builder`s, `medication_reminder_eligibility_query`, `medication_finder_search_responder`.
9. **Presenter/helper:** `schedules/card_presenter`, `application_helper`.
10. **Playwright/system:** audit `spec/system` & `spec/features` for untested user journeys (e.g. full invite-acceptance flow, carer-relationship management) once unit coverage is solid.

When the suite is mature, flip the CI `mutation` job from `continue-on-error: true` to blocking, scoped to changed subjects (decision #3 → hard gate).
