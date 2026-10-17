# How to review with the checklist

Use the [review checklist](review-checklist.md) on every Wear OS PR. This page shows how to apply it and what good review comments look like.

## When to use it

- Every PR that touches UI, state, data, or background work on the watch app.
- Quick pass: product constraints, correctness, wearable UX.
- Deeper pass: performance/battery, data layer (Room/DataStore, query scope).

## How to run the checklist

1. **Product constraints**  
   Confirm the primary flow is still fast and watch-appropriate. If the PR adds text inputs or blocking dialogs, flag them and suggest alternatives (presets, +/- , confirmations only for destructive actions).

2. **Correctness**  
   Check navigation/back behavior, persistence across process death, and that timers/background work stop when the user leaves the screen. Look for “latest record” or “last X” logic and ensure it’s not stale.

3. **Wearable UX**  
   Primary action should be the biggest target. No dense clusters of small buttons. Text should be minimal and glanceable.

4. **Performance and battery**  
   No IO on the main thread. Timers and sensors must be cancelled in the right lifecycle (e.g. when leaving the screen). Avoid heavy recomposition and haptic spam.

5. **Data layer**  
   Room for history/relational data, DataStore for settings. Queries should load only what the UI needs (e.g. “latest record” not full history).

## Example PR comments

Use short, watch-specific feedback so the author knows exactly what to fix.

| Situation | Example comment |
|-----------|------------------|
| Text input on watch | "This adds a text input; can we use presets or +/- instead to avoid typing on watch?" |
| Timer keeps running after exit | "Timer keeps ticking after leaving the screen; cancel the job on dispose/navigation." |
| Query loads too much | "This query loads full history; can we fetch only what the UI needs (e.g. latest record)?" |
| Tiny tap targets | "These buttons are too small for one-thumb use; can we make the primary action the largest target?" |
| Blocking confirmation | "Primary action is behind a confirmation dialog; can we make it one tap and only confirm for destructive actions?" |
| Main-thread DB access | "This read runs on the main thread; move it to a coroutine / Dispatchers.IO." |

Copy-paste template for review notes: [PR review template](templates/pr-review.md).
