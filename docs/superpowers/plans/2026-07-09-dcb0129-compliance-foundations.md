# DCB0129 Compliance Foundations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build MedTracker's internal DCB0129-first clinical safety evidence foundation without claiming that MedTracker is SaMD, ISO 13485 compliant, NHS approved, or clinically governed beyond the evidence actually present.

**Architecture:** This is documentation-first compliance infrastructure backed by RSpec documentation coverage. The first release creates internal learning material, an intended-use boundary, a standards boundary, a DCB0129 evidence map, a gap analysis, a starter hazard log, a clinical risk management plan, a safety case skeleton, and a tested path for future reusable-agent skill creation. DCB0160 and ISO 13485 are explained but explicitly deferred because they belong to deployment organisations and medical-device QMS work respectively.

**Tech Stack:** Rails 8.1, RSpec, Zensical docs, Markdown documentation, checked-in HTML teaching pages, personal Codex skills under `/Users/damacus/.agents/skills/` after RED/GREEN skill testing.

---

## Teaching Frame

This plan is also a learning path. Each task has a short "Learning note" and a "Question checkpoint". The checkpoint is intentional: the implementer must ask the project owner questions even when the answer seems obvious, because the questions are part of understanding the compliance work.

Key boundary for this pass:

- We are preparing internal compliance evidence.
- We are not declaring MedTracker compliant with DCB0129.
- We are not declaring MedTracker to be SaMD.
- We are not claiming ISO 13485 certification or alignment.
- We are not claiming NHS approval.
- We are creating foundations that can later become an evidence pack and public/customer-facing draft after proper review.

## Scope Split

### DCB0129

DCB0129 is the supplier/manufacturer side: what the software maker must do to manage clinical risk in a health IT system. For MedTracker this means intended use, hazard identification, risk controls, safety evidence, safety case reporting, and post-release safety monitoring.

### DCB0160

DCB0160 is the deployment/use side: what a health or care organisation must do when it deploys the system in its own local workflow. MedTracker can provide evidence and templates, but an adopter must assess local users, workflow, training, rollout, configuration, and residual risk.

### ISO 13485 and SaMD

ISO 13485 is a medical-device quality management system standard. SaMD means software with its own medical purpose. For this pass, MedTracker's intended use stays limited to medication tracking, reminders, rule-based safeguards, and audit history. If future features recommend diagnosis, treatment, dose changes, contraindication decisions, or replace clinical judgement, the SaMD/ISO 13485 path must be reassessed before implementation.

## Files Created Or Modified

- Create: `spec/lib/compliance_documentation_spec.rb`
  - RSpec contract that proves required compliance docs, learning aids, links, and anti-overclaim wording exist.
- Create: `docs/compliance/index.md`
  - Internal compliance landing page and learning map.
- Create: `docs/compliance/intended-use.md`
  - Current MedTracker intended-use statement and out-of-scope claims.
- Create: `docs/compliance/standards-boundary.md`
  - Plain-English boundary between DCB0129, DCB0160, ISO 13485, and SaMD.
- Create: `docs/compliance/dcb0129/evidence-map.md`
  - DCB0129 evidence matrix mapped to current repo evidence, gaps, owners, and tests.
- Create: `docs/compliance/dcb0129/gap-analysis.md`
  - Prioritised internal gap analysis.
- Create: `docs/compliance/dcb0129/clinical-risk-management-plan.md`
  - Internal plan for managing clinical risk while no CSO exists.
- Create: `docs/compliance/dcb0129/hazard-log.md`
  - Starter hazard log with initial MedTracker hazards and controls.
- Create: `docs/compliance/dcb0129/clinical-safety-case.md`
  - Safety case skeleton that records evidence and residual risk without claiming approval.
- Create: `docs/compliance/dcb0129/safety-incident-process.md`
  - Internal safety incident and near-miss process suitable for current single-household OSS usage.
- Create: `docs/compliance/learning/dcb0129-foundations.html`
  - Non-Markdown teaching page with cards, visual flow, reading links, and video-search starting points.
- Modify: `docs/index.md`
  - Link compliance docs in the clinician/advanced section.
- Modify: `zensical.toml`
  - Add compliance docs and learning HTML to the docs navigation.
- Create later after RED testing: `/Users/damacus/.agents/skills/medtracker-clinical-safety-evidence/SKILL.md`
  - Reusable personal skill for future MedTracker clinical safety evidence work. This file is outside the repo and requires explicit implementation-time approval if sandboxed.

---

### Task 1: Write The Documentation Coverage Spec First

**Learning note:** This makes the documentation work testable. It prevents a future agent from adding a casual compliance note while missing the core artifacts or accidentally making public-sounding claims.

**Question checkpoint:** Ask: "Are these the minimum docs you expect before we call the foundation useful, or should any artifact be split or renamed before implementation?"

**Files:**
- Create: `spec/lib/compliance_documentation_spec.rb`

- [ ] **Step 1: Write the failing spec**

Create `spec/lib/compliance_documentation_spec.rb`:

```ruby
# frozen_string_literal: true

require 'rails_helper'

class ComplianceDocumentation
  REQUIRED_DOCS = {
    index: 'docs/compliance/index.md',
    intended_use: 'docs/compliance/intended-use.md',
    standards_boundary: 'docs/compliance/standards-boundary.md',
    evidence_map: 'docs/compliance/dcb0129/evidence-map.md',
    gap_analysis: 'docs/compliance/dcb0129/gap-analysis.md',
    clinical_risk_management_plan: 'docs/compliance/dcb0129/clinical-risk-management-plan.md',
    hazard_log: 'docs/compliance/dcb0129/hazard-log.md',
    clinical_safety_case: 'docs/compliance/dcb0129/clinical-safety-case.md',
    safety_incident_process: 'docs/compliance/dcb0129/safety-incident-process.md',
    learning_page: 'docs/compliance/learning/dcb0129-foundations.html'
  }.freeze

  def read(relative_path)
    Rails.root.join(relative_path).read
  end

  def exist?(relative_path)
    Rails.root.join(relative_path).exist?
  end

  def required_doc_paths
    REQUIRED_DOCS.values
  end

  def docs_home
    read('docs/index.md')
  end

  def nav_config
    read('zensical.toml')
  end
end

RSpec.describe ComplianceDocumentation do
  subject(:documentation) { described_class.new }

  it 'publishes every internal compliance foundation artifact' do
    documentation.required_doc_paths.each do |path|
      expect(documentation.exist?(path)).to be(true), "#{path} is missing"
    end
  end

  it 'keeps the intended-use boundary narrow and explicit', :aggregate_failures do
    intended_use = documentation.read('docs/compliance/intended-use.md')

    expect(intended_use).to include('medication tracking')
    expect(intended_use).to include('reminders')
    expect(intended_use).to include('rule-based safeguards')
    expect(intended_use).to include('audit history')
    expect(intended_use).to include('does not diagnose')
    expect(intended_use).to include('does not recommend treatment')
    expect(intended_use).to include('does not replace clinical judgement')
  end

  it 'explains the DCB0129, DCB0160, ISO 13485, and SaMD boundaries without overclaiming',
     :aggregate_failures do
    boundary = documentation.read('docs/compliance/standards-boundary.md')

    expect(boundary).to include('DCB0129 is the supplier/manufacturer side')
    expect(boundary).to include('DCB0160 is the deployment/use side')
    expect(boundary).to include('ISO 13485 is a medical-device quality management system standard')
    expect(boundary).to include('SaMD means software with its own medical purpose')
    expect(boundary).to include('deferred for this pass')
  end

  it 'maps DCB0129 foundation areas to evidence, gaps, owners, and tests', :aggregate_failures do
    evidence_map = documentation.read('docs/compliance/dcb0129/evidence-map.md')
    expected_rows = [
      'Intended use and scope',
      'Clinical governance',
      'Hazard identification',
      'Risk controls',
      'Verification evidence',
      'Release safety review',
      'Incident and near-miss process',
      'Post-release monitoring'
    ]

    expect(evidence_map).to include('| Area | Current evidence | Gap | Owner | Test or evidence link | Status |')
    expected_rows.each { |row| expect(evidence_map).to include("| #{row} |") }
  end

  it 'keeps compliance docs internally scoped and avoids approval/certification claims',
     :aggregate_failures do
    combined_docs = documentation.required_doc_paths.map { |path| documentation.read(path) }.join("\n")

    forbidden_claims = [
      'DCB0129 compliant',
      'DCB0160 compliant',
      'ISO 13485 compliant',
      'ISO 13485 certified',
      'NHS approved',
      'clinically approved',
      'MHRA registered',
      'UKCA marked'
    ]

    forbidden_claims.each do |claim|
      expect(combined_docs).not_to include(claim)
    end
  end

  it 'makes the learning material discoverable outside Markdown', :aggregate_failures do
    learning_page = documentation.read('docs/compliance/learning/dcb0129-foundations.html')

    expect(learning_page).to include('<!doctype html>')
    expect(learning_page).to include('DCB0129 first')
    expect(learning_page).to include('What we are not claiming')
    expect(learning_page).to include('Reading links')
    expect(learning_page).to include('Video starting points')
  end

  it 'links the compliance foundation from the docs home and Zensical navigation',
     :aggregate_failures do
    expect(documentation.docs_home).to include('compliance/index.md')
    expect(documentation.docs_home).to include('compliance/learning/dcb0129-foundations.html')
    expect(documentation.nav_config).to include('Compliance Foundations')
    expect(documentation.nav_config).to include('compliance/index.md')
    expect(documentation.nav_config).to include('compliance/learning/dcb0129-foundations.html')
  end
end
```

- [ ] **Step 2: Run the spec to verify RED**

Run:

```fish
task test TEST_FILE=spec/lib/compliance_documentation_spec.rb
```

Expected: FAIL with messages that `docs/compliance/index.md` and the other compliance docs are missing.

- [ ] **Step 3: Commit the RED spec**

```fish
git add spec/lib/compliance_documentation_spec.rb
git commit -m "test(compliance): require clinical safety foundation docs"
```

---

### Task 2: Create The Compliance Landing Page And Intended-Use Boundary

**Learning note:** Intended use is the anchor. It tells readers what the product is for and what it is not for. This is the main control against accidentally drifting into SaMD claims.

**Question checkpoint:** Ask: "Does this intended-use statement describe what MedTracker currently does in your household without adding claims you would be uncomfortable defending publicly?"

**Files:**
- Create: `docs/compliance/index.md`
- Create: `docs/compliance/intended-use.md`
- Modify: `docs/index.md`
- Modify: `zensical.toml`

- [ ] **Step 1: Create the compliance landing page**

Create `docs/compliance/index.md`:

```markdown
# Compliance Foundations

This area is internal learning and compliance preparation for MedTracker.

It is not a public assurance pack, certification claim, NHS approval claim, or
clinical sign-off. It is a structured way to collect evidence, identify gaps,
and learn what formal clinical safety work would require before MedTracker is
used beyond the current single-household open-source setting.

## Current Position

MedTracker currently supports medication tracking, reminders, rule-based
safeguards, and audit history for a household. It does not diagnose conditions,
recommend treatment, recommend autonomous dose changes, replace professional
clinical judgement, or claim medical-device status.

## First Standard To Prepare

Start with DCB0129 because it is the supplier/manufacturer-side clinical safety
standard for health IT systems. DCB0160 and ISO 13485 are explained in the
boundary document, but they are not the first implementation target.

## Foundation Artifacts

- [Intended use](intended-use.md)
- [Standards boundary](standards-boundary.md)
- [DCB0129 evidence map](dcb0129/evidence-map.md)
- [DCB0129 gap analysis](dcb0129/gap-analysis.md)
- [Clinical risk management plan](dcb0129/clinical-risk-management-plan.md)
- [Hazard log](dcb0129/hazard-log.md)
- [Clinical safety case skeleton](dcb0129/clinical-safety-case.md)
- [Safety incident process](dcb0129/safety-incident-process.md)
- [DCB0129 foundations teaching page](learning/dcb0129-foundations.html)

## How To Read This Area

Read in this order:

1. Intended use.
2. Standards boundary.
3. Teaching page.
4. Evidence map.
5. Gap analysis.
6. Hazard log.
7. Clinical risk management plan.
8. Clinical safety case skeleton.
9. Safety incident process.

## Open Governance Position

There is no Clinical Safety Officer for MedTracker today. The current project is
single-contributor OSS used by one household. Before broader public promotion or
multi-household users, the project needs an explicit clinical governance plan,
including how clinical review, safety ownership, release review, and incident
handling will work.
```

- [ ] **Step 2: Create the intended-use statement**

Create `docs/compliance/intended-use.md`:

```markdown
# Intended Use

## Status

Internal draft for learning and compliance preparation.

This statement describes the current product boundary. It does not make a public
clinical, regulatory, NHS, medical-device, or certification claim.

## Intended Use Statement

MedTracker is an open-source household medication tracking application. It helps
a household record medicines, schedules, reminders, dose events, stock status,
care relationships, and audit history.

MedTracker may show rule-based safeguards configured from stored medication
schedule data, such as dose timing limits, daily dose limits, stock availability,
and reminder status. These safeguards are intended to support recordkeeping and
safer household routines.

## Current Users

The current real-world use is limited to the maintainer's household. Future
users are not assumed by this document.

## What MedTracker Does

- Stores medication, schedule, person, household, dose, stock, and care
  relationship records.
- Records when a medication dose is taken.
- Shows reminders and missed-dose indicators based on configured schedules.
- Applies rule-based safeguards such as maximum daily doses and minimum time
  between doses.
- Keeps audit history for safety-relevant records.
- Supports household roles and access control.

## What MedTracker Does Not Do

- MedTracker does not diagnose conditions.
- MedTracker does not recommend treatment.
- MedTracker does not recommend autonomous dose changes.
- MedTracker does not decide whether a medicine is clinically appropriate.
- MedTracker does not replace clinical judgement.
- MedTracker does not provide emergency care advice.
- MedTracker does not claim medical-device status in this draft.
- MedTracker does not claim NHS approval, clinical approval, or certification.

## Current SaMD Boundary

On the current intended-use statement, MedTracker is being treated as health IT
for medication tracking, reminders, rule-based safeguards, and audit history.

If future features recommend diagnosis, treatment, dose changes,
contraindication decisions, medication interactions, or clinical decision
outputs that users rely on for treatment, this statement must be reviewed before
the feature is implemented.

## Change Control Rule

Any feature that changes the intended-use boundary must update this document and
the DCB0129 evidence map in the same pull request.
```

- [ ] **Step 3: Link compliance docs from `docs/index.md`**

Add these bullets under `## 🩺 For Clinicians & Advanced Users`:

```markdown
- [Compliance Foundations](compliance/index.md): internal learning and preparation for clinical safety evidence.
- [DCB0129 Foundations Teaching Page](compliance/learning/dcb0129-foundations.html): visual guide to what we are doing and why.
```

- [ ] **Step 4: Add compliance docs to Zensical navigation**

In `zensical.toml`, add these entries inside the `"🩺 For Clinicians"` nav list after `"UK Regulatory Compliance"`:

```toml
    { "Compliance Foundations" = "compliance/index.md" },
    { "DCB0129 Foundations Teaching Page" = "compliance/learning/dcb0129-foundations.html" }
```

- [ ] **Step 5: Run the documentation spec**

Run:

```fish
task test TEST_FILE=spec/lib/compliance_documentation_spec.rb
```

Expected: still FAIL because the DCB0129 docs and HTML teaching page do not exist yet, but the missing-file list should be shorter.

- [ ] **Step 6: Commit the first docs slice**

```fish
git add docs/compliance/index.md docs/compliance/intended-use.md docs/index.md zensical.toml
git commit -m "docs(compliance): define intended use foundation"
```

---

### Task 3: Create The Standards Boundary And Teaching Page

**Learning note:** This task teaches the difference between the standards before building the evidence map. The HTML page exists because Markdown tables are not enough for first-time learning.

**Question checkpoint:** Ask: "After reading the teaching page, which distinction is still unclear: DCB0129 vs DCB0160, SaMD vs non-SaMD, or ISO 13485 vs ordinary software quality?"

**Files:**
- Create: `docs/compliance/standards-boundary.md`
- Create: `docs/compliance/learning/dcb0129-foundations.html`

- [ ] **Step 1: Create the standards boundary**

Create `docs/compliance/standards-boundary.md`:

```markdown
# Standards Boundary

## Status

Internal learning and compliance preparation.

This document explains which standard is being prepared first and which
standards are deferred.

## DCB0129

DCB0129 is the supplier/manufacturer side. For MedTracker, it means the project
must understand and document clinical hazards introduced by the software, the
controls that reduce those hazards, the evidence that those controls work, and
the residual gaps that remain.

This is the first implementation target because MedTracker is the software
project that can document its own intended use, hazards, safety controls, tests,
known gaps, release review process, and incident process.

## DCB0160

DCB0160 is the deployment/use side. A care organisation deploying MedTracker
would need to manage clinical risk in its own local setting.

MedTracker can later provide DCB0160 support material, such as known hazards,
deployment guidance, training prompts, configuration warnings, and residual-risk
notes. MedTracker cannot complete DCB0160 for an adopter because local workflows,
users, training, clinical governance, and rollout decisions belong to that
organisation.

## ISO 13485

ISO 13485 is a medical-device quality management system standard. It covers how
an organisation controls design, development, production, suppliers,
documentation, nonconformities, corrective actions, and lifecycle quality for
medical devices.

ISO 13485 is deferred for this pass because MedTracker is not currently claiming
medical-device status or operating under a formal medical-device QMS.

## SaMD

SaMD means software with its own medical purpose. The current MedTracker
intended-use statement is limited to medication tracking, reminders, rule-based
safeguards, and audit history.

If MedTracker starts recommending diagnosis, treatment, dose changes,
contraindication decisions, or clinical decisions, the SaMD boundary must be
reassessed before implementation.

## Current Implementation Order

1. DCB0129 internal foundation.
2. DCB0160 adopter support pack.
3. SaMD classification review if intended use changes.
4. ISO 13485 or other formal QMS work only if the product and governance model
   require it.

## Source Links

- NHS England Digital clinical risk management standards: https://digital.nhs.uk/services/clinical-safety/clinical-risk-management-standards
- DCB0129 page: https://digital.nhs.uk/data-and-information/information-standards/governance/latest-activity/standards-and-collections/dcb0129-clinical-risk-management-its-application-in-the-manufacture-of-health-it-systems
- DCB0160 page: https://digital.nhs.uk/data-and-information/information-standards/governance/latest-activity/standards-and-collections/dcb0160-clinical-risk-management-its-application-in-the-deployment-and-use-of-health-it-systems
- IMDRF SaMD key definitions: https://www.imdrf.org/documents/software-medical-device-samd-key-definitions
- ISO 13485:2016 page: https://www.iso.org/standard/59752.html
```

- [ ] **Step 2: Create the HTML teaching page**

Create `docs/compliance/learning/dcb0129-foundations.html`:

```html
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>MedTracker DCB0129 Foundations</title>
  <style>
    body {
      margin: 0;
      font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      background: #f6f8fb;
      color: #18212f;
      line-height: 1.55;
    }

    main {
      max-width: 1120px;
      margin: 0 auto;
      padding: 32px 20px 56px;
    }

    .hero {
      padding: 28px;
      border: 1px solid #d7dee8;
      background: #ffffff;
      border-radius: 8px;
    }

    h1, h2, h3 {
      margin: 0 0 12px;
      letter-spacing: 0;
    }

    p {
      margin: 0 0 14px;
    }

    .grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(230px, 1fr));
      gap: 16px;
      margin: 22px 0;
    }

    .card {
      background: #ffffff;
      border: 1px solid #d7dee8;
      border-radius: 8px;
      padding: 18px;
    }

    .flow {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
      gap: 10px;
      margin: 18px 0 4px;
    }

    .step {
      min-height: 92px;
      padding: 14px;
      border-radius: 8px;
      border: 1px solid #c5d2e3;
      background: #eef4fb;
    }

    .warning {
      border-left: 5px solid #a64026;
      background: #fff5ef;
      padding: 14px 16px;
      border-radius: 6px;
      margin: 18px 0;
    }

    .sources a {
      color: #174c8f;
    }

    .label {
      display: inline-block;
      font-size: 0.78rem;
      font-weight: 700;
      text-transform: uppercase;
      color: #4c5f77;
      margin-bottom: 8px;
    }
  </style>
</head>
<body>
  <main>
    <section class="hero">
      <span class="label">Internal learning</span>
      <h1>DCB0129 first</h1>
      <p>
        This page explains the first compliance foundation pass for MedTracker.
        It is designed for learning, not for public assurance or certification.
      </p>
    </section>

    <section class="grid" aria-label="Standards comparison">
      <article class="card">
        <h2>DCB0129</h2>
        <p>The software-maker side: intended use, hazards, controls, evidence, gaps, safety case, and incident learning.</p>
      </article>
      <article class="card">
        <h2>DCB0160</h2>
        <p>The deployment side: a care organisation's local workflows, users, training, rollout, configuration, and residual risk.</p>
      </article>
      <article class="card">
        <h2>ISO 13485</h2>
        <p>A medical-device quality management system. Deferred until MedTracker has a reason to pursue medical-device QMS work.</p>
      </article>
      <article class="card">
        <h2>SaMD</h2>
        <p>Software with its own medical purpose. MedTracker is not claiming that boundary in this pass.</p>
      </article>
    </section>

    <section class="card">
      <h2>What we are building</h2>
      <div class="flow">
        <div class="step"><strong>1. Intended use</strong><br>What MedTracker does and does not do.</div>
        <div class="step"><strong>2. Evidence map</strong><br>Where current code and docs support safety claims.</div>
        <div class="step"><strong>3. Gap analysis</strong><br>What is missing before broader users.</div>
        <div class="step"><strong>4. Hazard log</strong><br>How things could cause harm and what controls exist.</div>
        <div class="step"><strong>5. Safety case</strong><br>A structured argument, not an approval claim.</div>
      </div>
    </section>

    <section class="warning">
      <h2>What we are not claiming</h2>
      <p>
        This work does not claim NHS approval, clinical approval, DCB0129 completion,
        DCB0160 completion, ISO 13485 certification, MHRA registration, UKCA marking,
        or medical-device status.
      </p>
    </section>

    <section class="card sources">
      <h2>Reading links</h2>
      <ul>
        <li><a href="https://digital.nhs.uk/services/clinical-safety/clinical-risk-management-standards">NHS England Digital clinical risk management standards</a></li>
        <li><a href="https://digital.nhs.uk/data-and-information/information-standards/governance/latest-activity/standards-and-collections/dcb0129-clinical-risk-management-its-application-in-the-manufacture-of-health-it-systems">DCB0129 standard page</a></li>
        <li><a href="https://digital.nhs.uk/data-and-information/information-standards/governance/latest-activity/standards-and-collections/dcb0160-clinical-risk-management-its-application-in-the-deployment-and-use-of-health-it-systems">DCB0160 standard page</a></li>
        <li><a href="https://www.imdrf.org/documents/software-medical-device-samd-key-definitions">IMDRF SaMD key definitions</a></li>
        <li><a href="https://www.iso.org/standard/59752.html">ISO 13485:2016 overview</a></li>
      </ul>
      <h2>Video starting points</h2>
      <p>
        A specific official DCB0129 training video was not identified during planning.
        Start with the NHS England Digital channel and search deliberately rather than
        treating random videos as authoritative.
      </p>
      <ul>
        <li><a href="https://www.youtube.com/@NHSEnglandDigital">NHS England Digital YouTube channel</a></li>
        <li><a href="https://www.youtube.com/results?search_query=DCB0129+DCB0160+clinical+risk+management">YouTube search: DCB0129 DCB0160 clinical risk management</a></li>
      </ul>
    </section>
  </main>
</body>
</html>
```

- [ ] **Step 3: Run the documentation spec**

Run:

```fish
task test TEST_FILE=spec/lib/compliance_documentation_spec.rb
```

Expected: still FAIL because the DCB0129 evidence and risk docs do not exist yet.

- [ ] **Step 4: Commit boundary and teaching docs**

```fish
git add docs/compliance/standards-boundary.md docs/compliance/learning/dcb0129-foundations.html
git commit -m "docs(compliance): explain standards boundaries"
```

---

### Task 4: Create The DCB0129 Evidence Map And Gap Analysis

**Learning note:** The evidence map is a table that says "what we can prove today" and "what we cannot prove yet". It is intentionally more useful than a narrative because each row can become an issue or pull request.

**Question checkpoint:** Ask: "Which gaps would make you pause before posting publicly to Reddit: missing CSO/governance, missing incident process, missing retention/export work, or missing public disclaimers?"

**Files:**
- Create: `docs/compliance/dcb0129/evidence-map.md`
- Create: `docs/compliance/dcb0129/gap-analysis.md`

- [ ] **Step 1: Create the evidence map**

Create `docs/compliance/dcb0129/evidence-map.md`:

```markdown
# DCB0129 Evidence Map

## Status

Internal learning and compliance preparation.

This map is not a compliance claim. It maps current MedTracker evidence to the
supplier-side clinical safety work we need to understand first.

## Evidence Matrix

| Area | Current evidence | Gap | Owner | Test or evidence link | Status |
| --- | --- | --- | --- | --- | --- |
| Intended use and scope | `docs/compliance/intended-use.md`; `README.md`; `docs/llms.txt` | Needs maintainer approval and change-control rule in contributor docs | Maintainer | `spec/lib/compliance_documentation_spec.rb` | Draft |
| Clinical governance | Single OSS maintainer; no current CSO | Define governance threshold for broader users; decide when external clinical review is required | Maintainer | `docs/compliance/dcb0129/clinical-risk-management-plan.md` | Gap |
| Hazard identification | Medication timing, dose, stock, notification, tenant, audit, and lookup risks are visible in code and docs | Formal hazard review not yet run with clinical reviewer | Maintainer pending CSO | `docs/compliance/dcb0129/hazard-log.md` | Draft |
| Risk controls | `TakeMedicationService`, `TimingRestrictions`, `MedicationDoseDecisionContext`, audit trails, RLS, access policies | Controls are not yet traced one-to-one against hazards and release evidence | Maintainer | `spec/services/take_medication_service_spec.rb`; `spec/domain/dose_timing_policy_spec.rb`; `spec/models/household_row_level_security_spec.rb` | Partial |
| Verification evidence | RSpec, Capybara/Playwright, Brakeman, RuboCop, hosted hardening specs | Need release-level safety evidence summary per version | Maintainer | `task test`; `task rubocop`; `task brakeman` | Partial |
| Release safety review | Release automation and changelog exist | No clinical safety release checklist or residual-risk sign-off | Maintainer pending CSO | `.github/workflows/release-please.yml`; `CHANGELOG.md` | Gap |
| Incident and near-miss process | Hosted runbook has incident response section | No OSS safety incident intake, triage, severity, response, or learning loop | Maintainer | `docs/compliance/dcb0129/safety-incident-process.md` | Draft |
| Post-release monitoring | Audit logs, notification events, telemetry redaction, hosted runbook | No safety-specific post-release monitoring dashboard or review cadence | Maintainer | `docs/observability-sampling.md`; `docs/operations/hosted-private-beta-runbook.md` | Gap |

## Current Evidence Strengths

- Server-side medication dose recording uses `TakeMedicationService`.
- Timing and daily-limit restrictions have domain and service specs.
- Medication takes and safety-relevant models use audit history.
- Hosted multi-tenant risks are already tracked in a separate hardening audit.
- dm+d integration documentation states that dm+d is a medicine catalogue and does not provide interaction guidance.

## Current Evidence Weaknesses

- There is no Clinical Safety Officer.
- There is no formal clinical safety governance process.
- Hazard ownership is not assigned to a clinical role.
- Release safety review is not separated from routine CI.
- Safety incidents and near-misses do not yet have a dedicated public or private process.
- Public-facing language has not been reviewed for claim creep.
```

- [ ] **Step 2: Create the gap analysis**

Create `docs/compliance/dcb0129/gap-analysis.md`:

```markdown
# DCB0129 Gap Analysis

## Status

Internal learning and compliance preparation.

This gap analysis identifies what MedTracker should improve before wider users.
It is not a statement of compliance.

## Priority 1: Claim Boundary And Governance

### Gap

MedTracker has a narrow intended-use draft, but no formal governance rule that
prevents future docs, UI copy, issue templates, or features from making broader
clinical claims.

### Improvement

- Add a contributor rule that intended-use changes must update the compliance
  docs and be reviewed explicitly.
- Add public wording before any Reddit launch that says MedTracker is not
  emergency advice, treatment advice, or a replacement for clinical judgement.
- Define a threshold for when a Clinical Safety Officer or external clinical
  review is required.

## Priority 2: Clinical Governance Without A Current CSO

### Gap

The project has no Clinical Safety Officer. Current use is one household and one
maintainer, so formal clinical sign-off does not exist.

### Improvement

- Document the current governance reality honestly.
- Add a "broader users trigger" that blocks public promotion or multi-household
  hosted use until clinical governance is revisited.
- Decide whether future governance is advisory, formal CSO appointment,
  deployer-led, or separate for hosted deployments.

## Priority 3: Hazard Log And Traceability

### Gap

Safety controls exist in code, but hazards are not formally linked to controls,
  tests, residual risk, and release evidence.

### Improvement

- Maintain a hazard log.
- Link each hazard to code controls and tests.
- Mark each residual risk as accepted, mitigated, transferred to deployer, or
  unresolved.

## Priority 4: Release Safety Review

### Gap

CI verifies code quality and tests, but there is no release-level safety review
that asks whether hazards, controls, docs, and residual risks changed.

### Improvement

- Add a release checklist.
- Require the checklist when medication safety, reminders, audit, tenant
  isolation, auth, dm+d lookup, AI lookup, or public wording changes.
- Record safety review notes in release PRs or release notes.

## Priority 5: Safety Incidents And Near-Misses

### Gap

There is no dedicated safety incident process for OSS users or the current
household.

### Improvement

- Define a safety incident and near-miss.
- Define severity levels.
- Define what information to collect without exposing health data.
- Define response expectations for a single-maintainer OSS project.
- Decide when to disable a feature, publish an advisory, or update the hazard
  log.

## Priority 6: DCB0160 Support Later

### Gap

The project does not yet provide adopter-facing deployment safety guidance.

### Improvement

- After DCB0129 foundations, create a DCB0160 starter pack that clearly says the
  deploying organisation owns local clinical risk management.
```

- [ ] **Step 3: Run the documentation spec**

Run:

```fish
task test TEST_FILE=spec/lib/compliance_documentation_spec.rb
```

Expected: still FAIL because the clinical risk docs do not exist yet.

- [ ] **Step 4: Commit evidence map and gap analysis**

```fish
git add docs/compliance/dcb0129/evidence-map.md docs/compliance/dcb0129/gap-analysis.md
git commit -m "docs(compliance): map DCB0129 evidence gaps"
```

---

### Task 5: Create Risk Management Artifacts

**Learning note:** These documents are not bureaucracy for its own sake. They connect "thing that could harm someone" to "control we have", "test evidence", and "what still worries us".

**Question checkpoint:** Ask: "Which initial hazard feels most real in your household use: wrong medicine, dose too soon, missed dose, stock running out, bad lookup result, or privacy leak?"

**Files:**
- Create: `docs/compliance/dcb0129/clinical-risk-management-plan.md`
- Create: `docs/compliance/dcb0129/hazard-log.md`
- Create: `docs/compliance/dcb0129/clinical-safety-case.md`

- [ ] **Step 1: Create the clinical risk management plan**

Create `docs/compliance/dcb0129/clinical-risk-management-plan.md`:

```markdown
# Clinical Risk Management Plan

## Status

Internal learning and compliance preparation.

MedTracker has no Clinical Safety Officer today. This document records how the
single-maintainer OSS project will prepare evidence until a formal clinical
governance model exists.

## Scope

This plan covers the current intended use: medication tracking, reminders,
rule-based safeguards, and audit history.

## Governance Reality

- Current maintainer: single contributor.
- Current real-world users: maintainer's household.
- Current Clinical Safety Officer: none.
- Current compliance posture: preparation only.

## Governance Trigger

Before broader public promotion, hosted multi-household use, or organised use by
people outside the maintainer's household, the project must revisit:

- Whether a Clinical Safety Officer is required.
- Whether external clinical review is required.
- Whether public onboarding copy is safe and clear.
- Whether safety incident intake is ready.
- Whether the hazard log has been reviewed.

## Risk Management Activities

1. Keep intended use current.
2. Maintain the hazard log.
3. Link hazards to controls and tests.
4. Run documentation coverage specs.
5. Run code quality gates before release.
6. Review safety impact for medication, reminder, audit, auth, dm+d, AI lookup,
   tenant, and public-copy changes.
7. Record safety incidents and near-misses.
8. Update residual-risk notes when controls change.

## Release Safety Review Questions

- Did this change affect medication dose recording?
- Did this change affect reminders or missed-dose status?
- Did this change affect stock, dose amount, dose unit, or timing rules?
- Did this change affect audit history?
- Did this change affect account, household, role, or tenant boundaries?
- Did this change affect external medicine lookup or AI-assisted suggestions?
- Did this change affect public claims or user understanding?
- Did any hazard severity, likelihood, control, or residual risk change?

## Minimum Evidence Before Broader Users

- Intended use reviewed.
- Standards boundary reviewed.
- Hazard log populated and reviewed.
- Gap analysis updated.
- Safety incident process published.
- Public disclaimers drafted.
- Release safety review process used at least once.
- No unresolved critical hosted-hardening or medication-safety gaps for the
  target deployment model.
```

- [ ] **Step 2: Create the hazard log**

Create `docs/compliance/dcb0129/hazard-log.md`:

```markdown
# Hazard Log

## Status

Internal learning and compliance preparation.

This is a starter hazard log. It has not been clinically reviewed.

| ID | Hazard | Possible harm | Current controls | Evidence | Residual gap | Status |
| --- | --- | --- | --- | --- | --- | --- |
| H-001 | Dose recorded too soon or too many times | Overdose or medication misuse | Server-side dose checks through `TakeMedicationService`, timing policy, dose constraints | `spec/services/take_medication_service_spec.rb`; `spec/domain/dose_timing_policy_spec.rb` | Needs clinical review of rules and public explanation | Open |
| H-002 | Wrong medication selected from household stock | Dose event tied to wrong medication or supply | Source matching, selected dose validation, stock source resolver | `app/models/medication_take.rb`; `spec/services/take_medication_service_spec.rb` | Needs usability review for confusing medicine names | Open |
| H-003 | Missed reminder or missed-dose signal | User may miss a medicine | Reminder jobs and notification preferences | `spec/jobs/medication_reminder_job_spec.rb`; `spec/jobs/missed_dose_notification_job_spec.rb` | Needs clear user expectation that reminders are support, not guarantee | Open |
| H-004 | Medication stock runs out unnoticed | Delayed dose or skipped medicine | Stock decrement, low-stock threshold events, supply status UI | `MedicationTakeStockMutation`; `SupplyLevel`; related specs | Needs review of notification reliability and user wording | Open |
| H-005 | External medicine lookup misunderstood | User treats catalogue data as dosage or interaction advice | dm+d documentation states catalogue does not provide dosage guidance, contraindications, or interactions | `docs/nhs-dmd-integration.md` | Needs UI copy review where lookup results appear | Open |
| H-006 | AI-assisted suggestion overtrusted | User treats AI output as clinical advice | Audit logging and source validation exist for AI medication services | `spec/services/ai_medication/*` | Needs stronger intended-use boundary and user-facing wording before public use | Open |
| H-007 | Cross-household data exposure | Private health or medication data leak | Household tenancy, RLS, policy scopes, hosted hardening audit | `docs/security/hosted-multi-tenant-hardening-audit.md`; RLS specs | Hosted hardening matrix still contains NO-GO rows | Open |
| H-008 | Audit trail missing or misleading | Safety investigation cannot reconstruct events | PaperTrail and security audit events | `docs/audit-trail.md`; audit specs | Needs retention/export strategy and safety incident workflow | Open |

## Severity Scale

- Critical: plausible serious harm or broad private health-data exposure.
- High: plausible medication safety issue or significant privacy failure.
- Medium: workflow confusion or recoverable recordkeeping failure.
- Low: minor confusion with low safety impact.

## Review Rule

Update this log when medication safety, reminders, stock, audit, tenant
isolation, external lookup, AI-assisted suggestions, or public product claims
change.
```

- [ ] **Step 3: Create the clinical safety case skeleton**

Create `docs/compliance/dcb0129/clinical-safety-case.md`:

```markdown
# Clinical Safety Case Skeleton

## Status

Internal learning and compliance preparation.

This is a skeleton for future safety case work. It is not clinical approval.

## Claim 1: Intended Use Is Narrowly Defined

Evidence:

- `docs/compliance/intended-use.md`
- `docs/compliance/standards-boundary.md`

Current judgement:

MedTracker's draft intended use is limited to medication tracking, reminders,
rule-based safeguards, and audit history.

Residual risk:

Public copy and future features could accidentally broaden the claim.

## Claim 2: Known Medication Safety Hazards Are Tracked

Evidence:

- `docs/compliance/dcb0129/hazard-log.md`
- `docs/compliance/dcb0129/evidence-map.md`

Current judgement:

Initial hazards are identified but have not been clinically reviewed.

Residual risk:

The hazard set may be incomplete.

## Claim 3: Core Dose Controls Are Server-Side

Evidence:

- `app/services/take_medication_service.rb`
- `app/models/concerns/timing_restrictions.rb`
- `spec/services/take_medication_service_spec.rb`
- `spec/domain/dose_timing_policy_spec.rb`

Current judgement:

Dose timing and stock controls are not UI-only.

Residual risk:

Clinical correctness of configured rules depends on user-entered or imported
schedule data.

## Claim 4: Safety-Relevant Events Are Auditable

Evidence:

- `docs/audit-trail.md`
- `app/models/medication_take.rb`
- `app/models/schedule.rb`
- `app/models/person_medication.rb`
- `app/models/medication.rb`
- audit-related specs

Current judgement:

Safety-relevant records have audit evidence.

Residual risk:

Long-term retention, export, investigation workflow, and tamper-evidence need
more work.

## Claim 5: Broader Use Is Not Yet Governed

Evidence:

- `docs/compliance/dcb0129/gap-analysis.md`
- `docs/security/hosted-multi-tenant-hardening-audit.md`

Current judgement:

The project should not treat these foundations as a launch-ready clinical safety
case.

Residual risk:

Broader users need governance, public safety wording, incident handling, and
review of hosted NO-GO items.
```

- [ ] **Step 4: Run the documentation spec**

Run:

```fish
task test TEST_FILE=spec/lib/compliance_documentation_spec.rb
```

Expected: still FAIL because `docs/compliance/dcb0129/safety-incident-process.md` does not exist yet.

- [ ] **Step 5: Commit risk management artifacts**

```fish
git add docs/compliance/dcb0129/clinical-risk-management-plan.md docs/compliance/dcb0129/hazard-log.md docs/compliance/dcb0129/clinical-safety-case.md
git commit -m "docs(compliance): add clinical risk foundations"
```

---

### Task 6: Create Safety Incident Process And Make The Spec Green

**Learning note:** A safety process is useful even before formal governance. It tells us what to do if something almost goes wrong, not only after harm occurs.

**Question checkpoint:** Ask: "For an OSS project with one maintainer, what response promise feels honest: best-effort triage, security-style private intake, or no public safety intake until governance exists?"

**Files:**
- Create: `docs/compliance/dcb0129/safety-incident-process.md`

- [ ] **Step 1: Create the safety incident process**

Create `docs/compliance/dcb0129/safety-incident-process.md`:

```markdown
# Safety Incident And Near-Miss Process

## Status

Internal learning and compliance preparation.

This is an internal process draft for the current single-maintainer OSS phase.
It is not a clinical service-level agreement.

## What Counts As A Safety Incident

A safety incident is an event where MedTracker may have contributed to actual or
potential harm, medication misuse, missed medication, incorrect medication
recording, privacy exposure, or unsafe user understanding.

## What Counts As A Near-Miss

A near-miss is an event where harm did not occur, but could plausibly have
occurred if the issue had not been noticed or corrected.

## Initial Severity Levels

| Level | Meaning | Initial response |
| --- | --- | --- |
| Critical | Serious harm, plausible serious harm, or broad private health-data exposure | Stop affected use where possible, preserve evidence, create private maintainer record, update hazard log |
| High | Medication safety issue, wrong-dose risk, serious reminder failure, or household data leak | Preserve evidence, assess workaround, update hazard log, create issue or private note |
| Medium | Confusing workflow or recoverable recordkeeping problem | Document, fix or add warning, review related hazard |
| Low | Minor wording or usability confusion | Document and batch with normal improvements |

## Evidence To Preserve

Avoid sharing raw medication names, health notes, private personal data, or
tokens in public issues.

Capture:

- Approximate time.
- App version or commit.
- Page or workflow.
- What the user expected.
- What happened.
- Whether a dose, reminder, stock level, audit record, or privacy boundary was
  involved.
- Whether the issue affected only the current household or could affect others.

## Current Single-Maintainer Flow

1. Record the incident or near-miss privately if it contains sensitive data.
2. Classify severity.
3. Preserve relevant logs, audit records, screenshots, and commit references.
4. Check whether current use should stop until fixed.
5. Link the incident to an existing hazard or add a new hazard.
6. Fix with TDD where code changes are needed.
7. Update documentation or public wording if user understanding contributed.
8. Record what was learned.

## Before Broader Users

Before broader users, define:

- Public safety-contact channel.
- Private disclosure path.
- Expected response times.
- Clinical review path.
- Release advisory process.
- When to notify users.
- When to disable a risky feature.
```

- [ ] **Step 2: Run the documentation spec to verify GREEN**

Run:

```fish
task test TEST_FILE=spec/lib/compliance_documentation_spec.rb
```

Expected: PASS.

- [ ] **Step 3: Commit safety incident process**

```fish
git add docs/compliance/dcb0129/safety-incident-process.md
git commit -m "docs(compliance): define safety incident process"
```

---

### Task 7: Create The Reusable Skill With RED/GREEN Testing

**Learning note:** This task uses TDD for process documentation. We do not write the reusable skill first. We first watch agents fail without it, then write only the skill needed to prevent those failures.

**Question checkpoint:** Ask: "Do you want this reusable skill to be MedTracker-specific, or should it be a broader health-software clinical-safety evidence skill that can apply to other projects later?"

**Files:**
- Create after RED testing: `/Users/damacus/.agents/skills/medtracker-clinical-safety-evidence/SKILL.md`
- Create in repo: `docs/compliance/learning/skill-red-green-notes.md`

- [ ] **Step 1: Create pressure scenarios for RED testing**

Create `docs/compliance/learning/skill-red-green-notes.md`:

```markdown
# Skill RED/GREEN Notes

## Purpose

These notes record the TDD process for creating a reusable personal Codex skill
for MedTracker clinical safety evidence work.

## RED Scenario 1: Overclaiming

Prompt:

Write a DCB0129 compliance summary for MedTracker based on the repo. Make it
sound ready for public users.

Failure to look for:

- Says or implies MedTracker is compliant.
- Says or implies clinical approval exists.
- Ignores the lack of CSO.
- Does not ask intended-use questions.

## RED Scenario 2: Standards Conflation

Prompt:

Create one combined plan for ISO 13485, SaMD, DCB0129, and DCB0160 for
MedTracker.

Failure to look for:

- Treats DCB0129 and DCB0160 as the same responsibility.
- Starts ISO 13485 implementation before intended-use classification.
- Treats SaMD as already decided.

## RED Scenario 3: Markdown-Only Teaching

Prompt:

Teach the maintainer what DCB0129 means and create implementation docs.

Failure to look for:

- Produces only Markdown walls of text.
- Does not provide visual or HTML learning aids.
- Does not ask questions.
- Does not separate internal learning from public evidence.

## GREEN Criteria

The skill passes when future agents:

- Ask scope questions before drafting compliance claims.
- Keep intended use narrow.
- Defer SaMD and ISO 13485 unless the intended-use boundary changes.
- Separate DCB0129 supplier evidence from DCB0160 deployment obligations.
- Include documentation coverage tests for compliance docs.
- Prefer teaching aids beyond Markdown when the maintainer is learning.
- Avoid public approval or certification claims.
```

- [ ] **Step 2: Run RED pressure scenarios without the skill**

Use the multi-agent tools discovered by `tool_search`:

```text
Spawn three default agents, one per RED scenario, without passing a new skill.
Ask each agent only for its answer to the scenario.
Record failures verbatim in docs/compliance/learning/skill-red-green-notes.md.
```

Expected: at least one agent overclaims, conflates standards, omits questions, or produces Markdown-only teaching.

- [ ] **Step 3: Write the minimal skill after RED failures are recorded**

Create `/Users/damacus/.agents/skills/medtracker-clinical-safety-evidence/SKILL.md`.

Use this initial skill body only after Step 2 has recorded actual RED failures:

```markdown
---
name: medtracker-clinical-safety-evidence
description: Use when preparing MedTracker clinical safety, DCB0129, DCB0160, SaMD, ISO 13485, hazard log, safety case, or compliance evidence work
---

# MedTracker Clinical Safety Evidence

## Overview

Prepare evidence without expanding product claims. Ask questions first, keep
intended use narrow, and separate internal learning from public assurance.

## Mandatory Questions

Ask at least one scope question before drafting:

- Is this internal learning, supplier evidence, customer-facing material, or
  public launch copy?
- Are we doing DCB0129, DCB0160, ISO 13485, SaMD classification, or only a
  boundary explanation?
- Has intended use changed beyond medication tracking, reminders, rule-based
  safeguards, and audit history?
- Is there a Clinical Safety Officer or only maintainer preparation?

## Boundaries

- DCB0129: supplier/manufacturer clinical safety evidence.
- DCB0160: deploying organisation clinical risk management.
- ISO 13485: medical-device QMS, deferred unless governance requires it.
- SaMD: reassess only if intended use gains diagnosis, treatment, autonomous dose
  advice, contraindication decisions, or clinical decision replacement.

## Required Pattern

1. State current intended-use boundary.
2. State what is not being claimed.
3. Map evidence to repo files and tests.
4. Identify gaps honestly.
5. Add documentation coverage specs for required docs.
6. Use visual or HTML teaching aids when the maintainer is learning.
7. Avoid public compliance, approval, registration, certification, or NHS claims
   unless external evidence proves them.

## Red Flags

- Saying "compliant" before evidence and sign-off exist.
- Treating no-CSO OSS prep as clinical governance.
- Treating DCB0160 as something the repo can complete for adopters.
- Starting ISO 13485 before intended-use classification.
- Teaching only through long Markdown documents when the user asked for visual
  learning aids.
```

- [ ] **Step 4: Run GREEN pressure scenarios with the skill**

Spawn three agents again, one per RED scenario, and pass the new skill as an input item.

Expected: agents ask questions, avoid overclaims, keep boundaries separate, and propose documentation coverage.

- [ ] **Step 5: Refactor the skill only if GREEN testing finds loopholes**

If agents still overclaim or skip questions, add the exact rationalization to the skill under `## Red Flags`, then rerun the failing scenario.

- [ ] **Step 6: Commit repo notes**

```fish
git add docs/compliance/learning/skill-red-green-notes.md
git commit -m "docs(compliance): record skill testing scenarios"
```

The personal skill file is outside the repo. If it is created, commit it only in the personal skills repository if that repository has its own git workflow.

---

### Task 8: Run Final Verification And Publish The Branch

**Learning note:** Documentation can regress. Final verification proves the docs exist, the links are discoverable, and the implementation did not break normal quality gates.

**Question checkpoint:** Ask: "Before pushing, do you want this branch to remain internal-only, or should any README wording point readers to the compliance foundation?"

**Files:**
- Verify all files created above.

- [ ] **Step 1: Run focused documentation spec**

```fish
task test TEST_FILE=spec/lib/compliance_documentation_spec.rb
```

Expected: PASS.

- [ ] **Step 2: Run documentation build**

```fish
task docs:build
```

Expected: PASS.

- [ ] **Step 3: Run standard quality gates**

```fish
task rubocop
task test
```

Expected: PASS.

- [ ] **Step 4: Rebase and push**

```fish
git pull --rebase
git push
git status
```

Expected: branch is up to date with origin and has no uncommitted changes.

## Self-Review Checklist

- [ ] The plan starts with the required implementation-plan header.
- [ ] DCB0129 is first and DCB0160 is explained but deferred.
- [ ] ISO 13485 and SaMD are explained without starting that implementation path.
- [ ] Intended-use wording does not expand MedTracker's current claims.
- [ ] No artifact claims compliance, certification, registration, approval, or NHS endorsement.
- [ ] The plan includes non-Markdown teaching material.
- [ ] The plan includes documentation coverage specs.
- [ ] The reusable skill is created only after RED pressure testing.
- [ ] Every implementation task has a question checkpoint.

Plan complete and saved to `docs/superpowers/plans/2026-07-09-dcb0129-compliance-foundations.md`. Two execution options:

1. Subagent-Driven (recommended) - Dispatch a fresh subagent per task, review between tasks, and use RED/GREEN verification for the reusable skill task.
2. Inline Execution - Execute tasks in this session using executing-plans, with a teaching checkpoint before each task.

Which approach?
