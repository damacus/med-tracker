# Implementation Plan: Docker Layer & Cache Optimization

## Technical Details

### 1. Pin `mailpit` Version
- Update `compose.yaml` to replace `axllent/mailpit:latest` with `axllent/mailpit:v1.29.1`.
- This ensures that all environments use a specific, stable version, preventing the `:latest` tag from pulling new, untagged images that take up disk space.

### 2. Refactor `Dockerfile`
- **Base Stage:** Create a `base` stage to handle shared OS dependencies (`libpq-dev`, `curl`), user/group creation, and `WORKDIR`.
- **Build Stage:** Use BuildKit cache mounts to speed up dependency installation:
  - `RUN --mount=type=cache,target=/usr/local/bundle bundle install`
  - `RUN --mount=type=cache,target=/usr/local/share/.cache/yarn yarn install`
- **Asset Precompilation Stage:** Move `rails assets:precompile` into its own stage that only copies asset-related files (`app/assets`, `app/javascript`, `config/`, etc.) to prevent frequent cache invalidation from Ruby code changes.
- **Production Stage:** Optimize the final `app` stage to copy only the minimal necessary artifacts from previous stages.

### 3. Optimize `.dockerignore`
- Add more exhaustive patterns to exclude non-essential files from the build context:
  - `conductor/`
  - `docs/`
  - `spec/` (for production builds)
  - `*.md` files (except those needed for the app)
  - `.git` and other metadata

## Tasks

### Phase 1: Preparation
- [ ] Verify `mailpit:v1.29.1` availability and suitability.
- [ ] Audit current `Dockerfile` for redundant instructions.

### Phase 2: Implementation
- [ ] Update `compose.yaml` with pinned `mailpit` version.
- [ ] Implement `base` stage in `Dockerfile`.
- [ ] Integrate BuildKit cache mounts for `bundle` and `yarn`.
- [ ] Restructure `Dockerfile` for granular asset precompilation.
- [ ] Update `.dockerignore` with optimized exclusions.

### Phase 3: Verification
- [ ] Build all profiles (`dev`, `test`, `prod`) to ensure they function.
- [ ] Run `task test` to verify no regressions in testing.
- [ ] Compare `docker system df` and build times before/after changes.

## Rollback Plan
- Revert changes to `Dockerfile` and `compose.yaml` to restore original build behavior.
- Prune images and volumes if necessary to recover space.
