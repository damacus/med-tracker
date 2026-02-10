# Documentation Screenshots

This directory stores screenshots referenced by user-facing documentation.

## Structure

- `post-implementation/`: product state captures
- `pr-425/`: historical UI captures retained for context
- `ui-audit/`: targeted UI review captures

## Rules

- Keep screenshots that support current docs.
- Delete screenshots older than 30 days unless still referenced by docs.
- Prefer PNG format.
- Use clear filenames that describe screen and state.

## Capture workflow

1. Start the app locally.
2. Navigate to the documented user flow.
3. Capture the smallest area that still shows the behavior clearly.
4. Save into an appropriate subdirectory under `docs/screenshots/`.

## Cleanup command

Review files older than 30 days:

```bash
find docs/screenshots -type f -mtime +30
```
