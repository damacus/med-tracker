# Performance & battery notes (Wear OS)

### Compose performance
- Prefer immutable UI state (`data class`) exposed via `StateFlow`.
- Be careful with allocations in composables (lists, maps, formatters) in hot paths.
- Use `remember` / `derivedStateOf` for derived values that are non-trivial.
- Keep recomposition-local state minimal; avoid duplicating sources of truth.

### Timers (rest)
- Default to **non-blocking** timers: the user should be able to continue the primary flow.
- Update cadence should be coarse (1s ticks is fine; avoid faster).
- Ensure timers are lifecycle-safe:
  - stop/cancel when leaving the relevant screen
  - avoid continuing work after the user leaves the primary flow

### Room + threading
- All DB operations should run off the main thread.
- Prefer "latest record" queries over pulling whole histories into UI.

### Haptics
- Use haptics intentionally:
  - on successful primary action completion
  - on timer completion (if used)
- Avoid repeated vibrations for intermediate timer ticks.

### Common watch pitfalls
- UI that looks fine on phone density but is cramped on watch: reduce text, enlarge controls.
- State resets due to navigation/back stack or process death: ensure state is restored where needed.
