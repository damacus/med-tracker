# Review checklist (Wear OS)

Use this when reviewing any PR/change for a Wear OS watch app.

### Product constraints
- [ ] The primary flow is fast and watch-appropriate (minimal steps, no friction)
- [ ] UI does not require precision typing; inputs are watch-friendly (presets, pickers, +/-)
- [ ] Primary actions are never blocked by confirmations/modals (except destructive actions)

### Correctness
- [ ] Navigation arguments/state restoration are correct (back behavior, deep links if any)
- [ ] Data persistence is correct and resilient (restart/process death where relevant)
- [ ] UI state updates only after successful operations (avoid "optimistic" bugs)
- [ ] Timers/background work behave as expected (non-blocking, cancel correctly)
- [ ] "Latest/Last" references are correct and not stale/cached incorrectly

### Wearable UX
- [ ] Primary action is the largest and easiest target
- [ ] Touch targets are large; no dense clusters of tiny buttons
- [ ] Text is minimal and glanceable (no paragraphs on watch)
- [ ] Defaults are sensible (user can complete core task immediately)
- [ ] Interaction works while moving (no precision gestures required)

### Performance and battery
- [ ] No IO on main thread; coroutines used appropriately
- [ ] Timers stop when leaving screen / when training ends (no runaway background work)
- [ ] Recomposition minimized in hot UI (avoid rebuilding lists/objects repeatedly)
- [ ] Haptics are not spammy (only for meaningful events)

### Data layer (Room / DataStore / Network)
- [ ] Room vs DataStore usage fits the problem (history/relational vs settings)
- [ ] Queries are minimal and well-scoped (avoid loading large histories into UI)
- [ ] "Latest record" queries are efficient (e.g., ORDER BY timestamp DESC LIMIT 1)
