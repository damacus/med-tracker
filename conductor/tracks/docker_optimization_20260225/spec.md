# Track Specification: Docker Layer & Cache Optimization

## Objective
Optimize Docker configuration to improve cache/layer reuse, reduce build times, and minimize disk footprint (specifically addressing reported 100GB reclaimed after pruning).

## Context
The project currently uses a multi-stage `Dockerfile` and `compose.yaml` with profiles (`dev`, `test`, `prod`). 
- Using `:latest` for `mailpit` causes silent updates and dangling images.
- `Dockerfile` layering is suboptimal: asset precompilation runs on every code change.
- `bundle install` and `yarn install` don't use BuildKit cache mounts.

## Requirements
- Pin `mailpit` to a stable version (`v1.29.1`).
- Refactor `Dockerfile` to consolidate base setup and optimize layering.
- Implement BuildKit cache mounts for gems and Node packages.
- Optimize `.dockerignore` to reduce build context size.
- Ensure all environments (`dev`, `test`, `prod`) remain functional and performant.

## Success Criteria
- Builds are faster when unrelated files change.
- `docker system df` shows reduced image size and cache footprint over time.
- No regression in developer workflow or test execution.
- `mailpit` images are reused across projects/worktrees.
