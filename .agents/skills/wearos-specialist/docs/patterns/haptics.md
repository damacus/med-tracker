# Pattern: Haptics on Wear OS

Use haptics for meaningful events only. Avoid spammy or repeated vibration so the watch stays pleasant and battery-friendly.

## When to use

- **Primary action success**: e.g. rep logged, set completed, goal hit.
- **Timer completion**: e.g. rest countdown finished (one clear pulse).
- **Critical feedback**: e.g. error that blocks the user (sparingly).

## When to avoid

- **Every timer tick**: Don’t vibrate on each second of a countdown.
- **Repeated rapid actions**: Don’t vibrate on every tap in a list or picker.
- **Decorative or non-essential feedback**: Prefer visual feedback when possible.

## Implementation

- Use the platform haptic APIs (e.g. `HapticFeedback` in Compose) with a single, short pattern for “success” or “complete.”
- Trigger haptics from the state holder (ViewModel) when the corresponding event occurs (e.g. after persisting the rep), not from the composable on every recomposition.

## Checklist

- [ ] Haptics only for meaningful events (success, timer done, critical error).
- [ ] No vibration on every tick of a timer or every list item.
- [ ] Haptic triggered from business logic, not from hot UI paths.
