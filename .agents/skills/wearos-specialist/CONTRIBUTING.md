# Contributing to Wear OS Specialist

Thanks for considering contributing. This repo is documentation- and guidance-focused: we welcome new checklist items, examples, performance notes, and pattern docs that help people build better Wear OS apps.

## How to propose changes

1. **Open an issue** (optional but helpful) to discuss larger changes or new sections.
2. **Fork the repo** and create a branch from `main`.
3. **Edit the docs** in `docs/` (canonical) or the root files where appropriate. Keep the [review checklist](docs/review-checklist.md) and [SKILL.md](SKILL.md) in sync if you change behavior or templates.
4. **Open a pull request** and fill out the PR template. Link any related issue.

## Content guidelines

- **Watch-first**: Guidance should assume the primary device is a watch (small screen, battery, lifecycle). Prefer presets/pickers over typing, glanceable UI, and lifecycle-safe timers/background work.
- **Battery and lifecycle**: Call out main-thread IO, runaway timers/sensors, and state restoration after process death where relevant.
- **Stack alignment**: We assume Kotlin, Jetpack Compose for Wear OS, coroutines/Flow, MVVM-style state holders, Room/DataStore, and Navigation. Mention alternatives only when they affect the guidance.

## Doc style conventions

- Use clear headings and short paragraphs. Tables are fine for checklists and indexes.
- Prefer “Wear OS” (capital O, capital S). Use “watch” for the device when the OS name isn’t needed.
- In code or templates, keep Kotlin/Compose examples consistent with the stack described in [SKILL.md](SKILL.md) (e.g. `StateFlow`, `data class` for UI state, DataStore for settings, Room for history).

## What we’re looking for

- New **checklist items** that catch real Wear OS issues (product, correctness, UX, performance, data layer).
- **Examples** (feature requests, review comments) that show the expected style and depth.
- **Pattern docs** (timers, state, haptics, Room queries, etc.) with concise, copy-paste-friendly guidance.
- **Fixes** for broken links, typos, or outdated Android/Wear OS advice.

If you’re unsure whether something fits, open an issue and we can align.
