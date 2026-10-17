# Pattern: Room queries on Wear OS

Keep queries minimal and scoped to what the UI needs. Prefer “latest record” or small windows over loading full history onto the watch.

## Rules of thumb

- **Latest record**: For “current” or “last” data, use a single query: `ORDER BY timestamp DESC LIMIT 1` (or equivalent). Expose via `Flow` so the UI updates when data changes.
- **Small history**: If the UI shows a short list (e.g. last 5 sessions), query only that window. Don’t load the full table.
- **Off main thread**: All Room operations must run off the main thread (e.g. coroutine on `Dispatchers.IO` or Room’s suspend/Flow).

## Implementation

- Dao: define a function that returns `Flow<Entity?>` or `Flow<List<Entity>>` with the right `ORDER BY` and `LIMIT`.
- ViewModel: collect the Flow in a scope that’s cancelled when the screen is gone; map to UI state and expose via StateFlow.
- Avoid one-off `runBlocking` or blocking calls from the UI layer.

## What to avoid

- Loading full history into memory when the UI only needs the latest or a small slice.
- Running queries on the main thread.
- Caching “latest” in a way that can become stale (e.g. not using Flow or not invalidating when data changes).

## Checklist

- [ ] Queries are scoped (latest record or limited list).
- [ ] All DB access is off the main thread.
- [ ] UI gets updates via Flow (or equivalent) so it stays in sync with DB changes.
