# Pattern: Timers on Wear OS

Countdown or elapsed timers (e.g. rest intervals) must be non-blocking, coarse, and lifecycle-safe so they don’t drain battery or keep running after the user leaves.

## Rules of thumb

- **Non-blocking**: The user can keep using the primary flow (e.g. log sets, navigate) while the timer runs. Don’t block the whole screen on the timer.
- **Coarse updates**: 1 second ticks are enough. Avoid sub-second updates; they cost CPU and wake-ups.
- **Lifecycle-safe**: Cancel the timer when the user leaves the relevant screen or flow. Don’t let jobs or `Flow` collectors keep running in the background.

## Implementation

- Use a coroutine (e.g. `delay(1_000)`) in a scope that’s cancelled when the screen/ViewModel is cleared.
- Expose remaining time via immutable UI state (e.g. `StateFlow<Int>`) and update once per second.
- If the app goes to background or the user navigates away, cancel the job so it doesn’t keep ticking.

## What to avoid

- Timers that tick faster than 1 s for purely UI countdown.
- Starting a timer without tying it to a lifecycle (ViewModel scope, or `DisposableEffect`/equivalent so it’s cancelled on dispose).
- Blocking the primary flow until the timer finishes (unless that’s an explicit product requirement).

## Checklist

- [ ] Timer updates at most once per second.
- [ ] Timer is cancelled when leaving the screen or when the flow ends.
- [ ] Primary flow remains usable while the timer runs (non-blocking).
- [ ] Remaining time is shown in a glanceable way (see [perf-battery](../perf-battery.md)).
