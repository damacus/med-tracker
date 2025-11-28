# Quick Start: Self-Hosted Runner

## 1. Create GitHub Token

```bash
# Go to: https://github.com/settings/tokens
# Create a token with 'repo' scope
```

## 2. Start the Runner

```bash
cd .github/runner

# Set your token
export GITHUB_TOKEN=ghp_your_token_here

# Start it up
./start-runner.sh
```

## 3. Verify

Visit: <https://github.com/damacus/med-tracker/settings/actions/runners>

You should see `med-tracker-runner` listed as "Idle" or "Active".

## 4. Test

Push a commit to trigger CI - it will now run on your local runner!

## Managing the Runner

```bash
# View logs
docker compose logs -f

# Stop runner
./stop-runner.sh

# Restart runner
docker compose restart
```

## Troubleshooting

**Runner not showing up?**

```bash
# Check logs
docker compose logs -f

# Verify token
echo $GITHUB_TOKEN
```

**Permission denied on Docker socket?**

```bash
sudo chmod 666 /var/run/docker.sock
```

See [README.md](README.md) for full documentation.
