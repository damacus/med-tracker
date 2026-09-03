# Team Recognition Record

## Standard

Award a medal only after an observable result is accepted. The citation must name the behaviour and the outcome it
protected or enabled. Attendance, starting work, routine completion, failed attempts, and rework do not qualify.

Recognition should be prompt, specific, and informational rather than controlling. Keep it proportional and give the
recipient a choice about whether public display is wanted. Do not turn medals into quotas, rankings, or a definition
of done. Corrective feedback remains separate from recognition.

For LLM agents, a medal is an operator-side credit record, not evidence that the model felt motivated. If feedback is
supplied to a model or used in training, use the accepted outcome plus a small number of observable process checks:
correct scope, safety invariants, review response, and reproducible verification. Do not optimise for medal count,
speed, verbosity, test volume, or merely following the workflow; those proxies invite gaming. State the defect and
the next invariant check immediately. Reserve recognition for the later accepted correction or result, not the rework.

This follows the evidence that positive feedback can support competence and autonomous motivation, while salient or
controlling rewards can undermine intrinsic motivation, particularly on complex work:

- [Self-Determination Theory in Work Organizations](https://doi.org/10.1146/annurev-orgpsych-032516-113108)
- [Criterion-specific reinforcement guidance](https://iris.peabody.vanderbilt.edu/mcontent/cs_parent/encouraging-appropriate-behavior/)
- [Verbal reward salience and complex tasks](https://onlinelibrary.wiley.com/doi/10.1002/job.2051)

LLM-agent evidence:

- [OpenAI: Improving mathematical reasoning with process supervision](https://openai.com/index/improving-mathematical-reasoning-with-process-supervision/)
- [Reflexion: Language Agents with Verbal Reinforcement Learning](https://arxiv.org/abs/2303.11366)
- [METR: Recent frontier models are reward hacking](https://metr.org/blog/2025-06-05-recent-reward-hacking/)

## Awards — 2026-09-02

### Hubble — Evidence Guardian

Awarded for identifying the missing P01B legacy reconciliation, retired-source filtering gap, migration/resume race,
and missing full-suite gates before acceptance. These findings prevented incorrect history and unsafe concurrent state
from entering the stack.

### Nightingale — Bounded Delivery

Awarded for delivering P01, P02, P01B, and P03 within their ownership boundaries, with observable behaviour tests and
reproducible verification. The medal recognises the accepted bounded results, not the rework encountered on the way.
The accepted layers remained 482, 296, and 194 changed lines for P02, P01B, and P03 respectively.

### Scout — Decomposition Map

Awarded for producing the dependency-aware 46-tranche map with owned paths and focused verification commands. The
map made the work independently reviewable and exposed shared-path collisions before dispatch.

## Non-awards

- No medal for turning up or being available.
- No medal for an initial attempt that required rework.
- No medal for passing through a tranche without a distinct, accepted contribution.

## Retro — 2026-09-02

### Improvement signals

- The team moved from a broad pause-period proposal to four accepted, composable implementation layers.
- Hubble's early plan review established one-writer ownership, independent review, TDD, line limits, and the wiring-test
  ban. Those controls were then reused consistently.
- Scout's decomposition exposed shared-path collisions before dispatch. This reduced the chance of concurrent edits to
  shared dashboard, report, and sync files.
- Nightingale's accepted tranches stayed bounded at 482, 296, and 194 changed lines, with focused and full-suite
  evidence recorded for each.
- P01B added a missing legacy-reconciliation layer after review identified a deployment-wide gap. That is recorded as
  a process improvement; the correction itself is not a separate medal.

### Handoff quality

Handoffs are currently healthy. Every accepted tranche leaves a branch, commit, report, review, checks, and dependency
state in `progress.md`. The next handoff names P04 as eligible and keeps P05R serialised behind its shared report paths.
The remaining friction is operational rather than behavioural: the container RuboCop command currently stops at Git's
safe-directory check, so that gate needs an environment fix before it can be treated as current evidence.

### Medal decisions

- Hubble: retain Evidence Guardian for the distinct, accepted safety findings.
- Nightingale: retain Bounded Delivery for the accepted bounded results only; do not count review rework.
- Scout: retain Decomposition Map for the accepted dependency map and collision detection.
- No additional medal for this retro, check rerun, availability, or routine handoff bookkeeping.
