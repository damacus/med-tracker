# How to debug on watch

When behavior is wrong or “weird” on device, follow this watch-first debugging workflow.

## 1. Reproduce on the watch

Prefer the real watch over the emulator. Note:

- Which screen and exact action sequence (taps, back, leave app).
- Watch state: AOD on/off, battery saver, connectivity (Wi‑Fi / Bluetooth / offline).
- Whether it happens every time or only after process death or long background.

## 2. Add temporary logs

Instrument the suspected areas:

- **Navigation**: log route/arguments when entering and when back is pressed.
- **Data**: log Room/DataStore/network reads and writes (values and timing).
- **Lifecycle**: log when timers, sensors, or coroutines start and stop; log `onCleared` / `DisposableEffect(onDispose)` or equivalent.

Keep logs minimal and remove them before merging.

## 3. Check common causes

| Symptom | What to check |
|--------|----------------|
| Wrong or stale data on screen | Main-thread DB/network; missing `Flow`/refresh; “latest record” query wrong or cached. |
| State resets after leaving app | No persistence (DataStore/Room) or no restoration in ViewModel/screen. |
| Timer or work continues after leaving screen | Job/coroutine/sensor not cancelled in `onCleared` or `onDispose`; no `cancel()` when navigating away. |
| Jank or freezes | IO or heavy work on main thread; large allocations in hot Compose paths. |
| Battery drain | Runaway timers, sensors, or polling; too-frequent updates (e.g. sub-second ticks). |

## 4. Fix and verify

- Fix one suspected cause, then reproduce again on the watch.
- Re-test after process death (e.g. force-stop and reopen) when the bug involves state or persistence.

For patterns that prevent these issues, see [Timers](patterns/timers.md), [State](patterns/state.md), and [Performance & battery](perf-battery.md).
