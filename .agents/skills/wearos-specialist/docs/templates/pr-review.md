# PR review template

Copy this when leaving a structured review on a Wear OS PR.

```markdown
## Correctness
- [ ] State transitions are correct under rapid taps / interruptions
- [ ] Data reads/writes are correct and resilient (restart, process death)

## Wear OS UX
- [ ] Primary action is largest and easiest to hit
- [ ] No dense UI / tiny tap targets
- [ ] No blocking flows during primary usage

## Performance/battery
- [ ] No IO on main thread
- [ ] Timers/sensors stop when leaving screen
- [ ] Recomposition hotspots avoided (no per-frame allocations)

## Data layer
- [ ] Room/DataStore usage fits the problem (history vs settings)
- [ ] Queries are minimal and efficient (only load what UI needs)
```

Add concrete comments under each section. See [How to review with the checklist](../how-to-review.md) for example comment phrasing.
