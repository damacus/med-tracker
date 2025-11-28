# Self-Hosted Runner Setup Summary

## What Was Created

### Runner Infrastructure

- **Dockerfile**: Ubuntu 24.04-based image with GitHub Actions runner
- **docker-compose.yml**: Service definition for running the runner
- **entrypoint.sh**: Auto-registration and cleanup script
- **start-runner.sh**: Easy startup script
- **stop-runner.sh**: Easy shutdown script

### Documentation

- **README.md**: Complete setup and troubleshooting guide
- **QUICKSTART.md**: Fast-track setup instructions
- **SETUP_SUMMARY.md**: This file

### CI Workflow Updates

Updated `.github/workflows/ci.yml` to use self-hosted runners:

```yaml
runs-on: [self-hosted, linux, x64, med-tracker]
```

All 5 jobs now use the self-hosted runner:

1. `scan_ruby` - Ruby security scan
2. `scan_js` - JavaScript security scan
3. `lint` - RuboCop code style check
4. `test_non_system` - RSpec unit/integration tests
5. `test_system` - Playwright system tests

## Next Steps

### 1. Create GitHub Personal Access Token

1. Visit: <https://github.com/settings/tokens>
2. Click "Generate new token (classic)"
3. Name: `med-tracker-runner`
4. Scope: ✅ `repo` (Full control of private repositories)
5. Generate and copy the token

### 2. Start the Runner

```bash
cd .github/runner
export GITHUB_TOKEN=your_token_here
./start-runner.sh
```

### 3. Verify Runner Registration

Visit: <https://github.com/damacus/med-tracker/settings/actions/runners>

You should see `med-tracker-runner` listed.

### 4. Test the Setup

```bash
# Commit and push the runner setup
git add .github/
git commit -m "feat: add self-hosted GitHub Actions runner"
git push

# This will trigger CI on your local runner
```

### 5. Monitor First Run

```bash
cd .github/runner
docker compose logs -f
```

## Benefits

✅ **No GitHub Actions minutes used** - Runs on your local machine
✅ **Faster builds** - No queue time, direct access to local Docker
✅ **Better caching** - Persistent Docker layers and bundler cache
✅ **Full control** - Debug and inspect builds easily
✅ **Cost effective** - Free for private repos

## Resource Requirements

- **CPU**: 2+ cores recommended
- **RAM**: 4GB+ recommended
- **Disk**: 20GB+ for Docker images and caches
- **Network**: Stable internet connection to GitHub

## Security Considerations

- ✅ Runner auto-registers and de-registers
- ✅ Token stored only in environment variables
- ✅ `.env` file gitignored
- ⚠️ Runner has Docker socket access (required for tests)
- ⚠️ Keep your machine secure (firewall, updates, etc.)

## Maintenance

### Update Runner Version

Edit `Dockerfile` and change `RUNNER_VERSION`:

```dockerfile
ARG RUNNER_VERSION=2.321.0  # Update this
```

Then rebuild:

```bash
docker compose build
docker compose up -d
```

### View Logs

```bash
docker compose logs -f
```

### Restart Runner

```bash
docker compose restart
```

### Stop Runner

```bash
./stop-runner.sh
```

## Troubleshooting

See [README.md](README.md#troubleshooting) for detailed troubleshooting steps.

Quick checks:

```bash
# Is Docker running?
docker info

# Is runner container running?
docker compose ps

# Check logs
docker compose logs -f

# Verify token
echo $GITHUB_TOKEN
```

## Reverting to GitHub-Hosted Runners

If you need to switch back:

```yaml
# In .github/workflows/ci.yml
runs-on: ubuntu-latest  # Change from [self-hosted, ...]
```

## Support

- GitHub Actions Docs: <https://docs.github.com/en/actions/hosting-your-own-runners>
- Runner Releases: <https://github.com/actions/runner/releases>
- Docker Docs: <https://docs.docker.com/>
