# Migration to Official Runner Image

## What Changed

We migrated from a custom-built Dockerfile to the official **myoung34/github-runner** image.

## Why?

✅ **Battle-tested** - Used by thousands of projects, well-maintained
✅ **No dependency guessing** - All GitHub Actions dependencies pre-installed
✅ **Auto-updates** - Pull latest image to get runner updates
✅ **Simpler setup** - No custom Dockerfile to maintain
✅ **Better support** - Active community and documentation

## What's Different

### Before (Custom Dockerfile)
- Had to manually install .NET Core dependencies
- Had to guess which packages were needed
- Had to maintain custom entrypoint script
- Required building image locally

### After (Official Image)
- All dependencies pre-installed
- Proven to work across many projects
- Uses official entrypoint with better error handling
- Just pull and run

## Configuration Changes

### docker-compose.yml

**Before:**
```yaml
services:
  runner:
    build:
      context: .
      dockerfile: Dockerfile
    environment:
      - GITHUB_TOKEN=${GITHUB_TOKEN}
      - GITHUB_REPOSITORY=${GITHUB_REPOSITORY}
```

**After:**
```yaml
services:
  runner:
    image: myoung34/github-runner:latest
    environment:
      - ACCESS_TOKEN=${GITHUB_TOKEN}
      - REPO_URL=https://github.com/${GITHUB_REPOSITORY}
```

### Environment Variables

| Old Variable | New Variable | Notes |
|--------------|--------------|-------|
| `GITHUB_TOKEN` | `ACCESS_TOKEN` | Same token, different name |
| `GITHUB_REPOSITORY` | `REPO_URL` | Now full URL format |
| `RUNNER_LABELS` | `LABELS` | Same format |
| N/A | `RUNNER_WORKDIR` | New: specify work directory |
| N/A | `RUNNER_GROUP` | New: runner group (default: Default) |

## Usage

Same as before, but faster:

```bash
cd .github/runner
export GITHUB_TOKEN=your_token_here
./start-runner.sh
```

The script now:
1. Pulls latest image (instead of building)
2. Starts container
3. Runner auto-registers

## Benefits

- **Faster startup** - No build time, just pull
- **Always up-to-date** - Pull latest for updates
- **More reliable** - Proven in production
- **Less maintenance** - No custom Dockerfile to update

## Rollback (if needed)

If you need to go back to custom Dockerfile:

1. Edit `docker-compose.yml`:
   ```yaml
   services:
     runner:
       build:
         context: .
         dockerfile: Dockerfile
       # ... rest of config
   ```

2. Rebuild:
   ```bash
   docker compose build
   docker compose up -d
   ```

## References

- Official image: <https://github.com/myoung34/docker-github-actions-runner>
- Documentation: <https://github.com/myoung34/docker-github-actions-runner/wiki>
- Docker Hub: <https://hub.docker.com/r/myoung34/github-runner>
