# Pattern: State on Wear OS

Keep a single source of truth, restore it after process death where it matters, and avoid duplicate or stale state so the watch UI stays correct and predictable.

## Rules of thumb

- **Single source of truth**: UI state lives in one place (e.g. ViewModel or state holder). Composables receive state and events; they don’t own business data.
- **Immutable UI state**: Use a `data class` (or similar) for the screen state and expose it via `StateFlow` (or Compose state). Avoid mutating the same object from multiple paths.
- **Restore after process death**: For anything the user expects to see again (e.g. current workout, settings), persist to DataStore or Room and restore when the screen/ViewModel is created.

## Implementation

- ViewModel (or equivalent) holds the canonical state, loads from DataStore/Room on start, and exposes `StateFlow<UiState>`.
- Composables use the state and callbacks for events (e.g. `onButtonClick()`). No business logic or persistence inside composables.
- For process death: load saved state in `init` or when the screen is created, and apply it to the initial state. Test by force-stopping the app and reopening.

## What to avoid

- Duplicating state (e.g. same value in ViewModel and in a composable `remember` that can get out of sync).
- Forgetting to persist and restore state that the user expects to survive restarts.
- “Optimistic” UI updates that don’t roll back on failure, leading to inconsistent state.

## Checklist

- [ ] One source of truth for screen state.
- [ ] UI state is immutable and exposed via Flow/StateFlow.
- [ ] State that must survive process death is persisted and restored.
- [ ] No business logic or persistence inside composables.
