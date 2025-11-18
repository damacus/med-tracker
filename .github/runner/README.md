# GitHub Actions Self-Hosted Runner

This directory contains the setup for running a self-hosted GitHub Actions runner in Docker.

## Prerequisites

- Docker and Docker Compose installed
- GitHub Personal Access Token (PAT) with `repo` scope

## Setup

### 1. Create GitHub Personal Access Token

1. Go to <https://github.com/settings/tokens>
2. Click "Generate new token (classic)"
3. Give it a descriptive name (e.g., "med-tracker-runner")
4. Select scopes:
   - ✅ `repo` (Full control of private repositories)
5. Click "Generate token"
6. **Copy the token immediately** (you won't be able to see it again)

### 2. Start the Runner

```bash
cd .github/runner

# Set your GitHub token
export GITHUB_TOKEN=your_token_here

# Start the runner
./start-runner.sh
```

The runner will:

- Build a Docker image with all necessary dependencies
- Register itself with the `damacus/med-tracker` repository
- Start listening for workflow jobs
- Automatically re-register if the container restarts

## Usage

### View Runner Logs

```bash
docker compose logs -f
```

### Check Runner Status

```bash
docker compose ps
```

### Stop the Runner

```bash
./stop-runner.sh
```

Or manually:

```bash
docker compose down
```

### Restart the Runner

```bash
docker compose restart
```

## Configuration

Environment variables can be set in `.env` file or passed directly:

- `GITHUB_TOKEN` - **Required**. GitHub PAT with `repo` scope
- `GITHUB_REPOSITORY` - Repository to register with (default: `damacus/med-tracker`)
- `RUNNER_NAME` - Name for the runner (default: `med-tracker-runner`)
- `RUNNER_LABELS` - Comma-separated labels (default: `self-hosted,linux,x64,med-tracker`)

Example `.env` file:

```bash
GITHUB_TOKEN=ghp_your_token_here
GITHUB_REPOSITORY=damacus/med-tracker
RUNNER_NAME=med-tracker-runner
RUNNER_LABELS=self-hosted,linux,x64,med-tracker
```

## Verifying the Runner

1. Go to <https://github.com/damacus/med-tracker/settings/actions/runners>
2. You should see your runner listed as "Idle" or "Active"

## Troubleshooting

### Runner not appearing in GitHub

- Check logs: `docker compose logs -f`
- Verify `GITHUB_TOKEN` has correct permissions
- Ensure token hasn't expired

### Docker socket permission denied

```bash
sudo chmod 666 /var/run/docker.sock
```

Or add your user to the docker group:

```bash
sudo usermod -aG docker $USER
newgrp docker
```

### Runner keeps restarting

- Check logs for errors
- Verify network connectivity to GitHub
- Ensure Docker has enough resources

## Security Notes

- **Never commit your `GITHUB_TOKEN` to git**
- The token is only stored in environment variables
- Runner automatically de-registers when stopped
- Use `.env` file (gitignored) for local development

## Architecture

The runner uses:

- **Base Image**: [myoung34/github-runner](https://github.com/myoung34/docker-github-actions-runner) (official community image)
- **GitHub Actions Runner**: Latest stable version (auto-updated)
- **Docker-in-Docker**: Mounts host Docker socket for running containers
- **Persistent Storage**: Runner work directory persisted in Docker volume
- **Pre-installed**: All GitHub Actions dependencies and common build tools
