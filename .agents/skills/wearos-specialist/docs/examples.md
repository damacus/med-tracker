# Examples

### Example: new feature request

Request: "Add a configurable countdown timer for a primary flow."

Expected response style:

```markdown
## Summary
- Add a configurable duration (seconds) with safe defaults
- Keep the primary flow non-blocking and watch-friendly

## Files to touch
- UI screen(s) for the flow
- State holder (ViewModel/state container)
- Persistence (DataStore for settings; Room if you need history/relations)

## Data/state changes
- UI state: `restSeconds`, `restRemainingSeconds`
- Events: `onRestDurationChange(delta)` or preset buttons
- Persistence: store duration in DataStore; keep migration simple

## Wear OS constraints check
- Glanceability: show remaining time clearly
- Tap targets: use +/- or presets (e.g., 30/60/90)
- Battery/lifecycle: timer cancels on exit; 1s ticks max

## Test plan
- Verify duration persists and applies on next use
- Verify timer cancels on navigation away and resumes correctly if intended
```

### Example: review comment style

Use short, watch-specific feedback:
- "This adds a text input; can we use presets or +/- instead to avoid typing on watch?"
- "Timer keeps ticking after leaving the screen; cancel the job on dispose/navigation."
- "This query loads full history; can we fetch only what the UI needs (e.g., latest record)?"
