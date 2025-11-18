#!/bin/bash
set -e

# Script to start the GitHub Actions self-hosted runner

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "=== GitHub Actions Self-Hosted Runner Setup ==="
echo ""

# Check if GITHUB_TOKEN is set
if [ -z "$GITHUB_TOKEN" ]; then
    echo "Error: GITHUB_TOKEN environment variable is not set."
    echo ""
    echo "To create a Personal Access Token (PAT):"
    echo "1. Go to https://github.com/settings/tokens"
    echo "2. Click 'Generate new token (classic)'"
    echo "3. Select scopes: 'repo' (full control of private repositories)"
    echo "4. Generate and copy the token"
    echo ""
    echo "Then run:"
    echo "  export GITHUB_TOKEN=your_token_here"
    echo "  ./start-runner.sh"
    exit 1
fi

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo "Error: Docker is not running. Please start Docker and try again."
    exit 1
fi

echo "Pulling latest runner image..."
docker compose pull

echo ""
echo "Starting runner..."
docker compose up -d

echo ""
echo "Runner started successfully!"
echo ""
echo "To view logs:"
echo "  docker compose logs -f"
echo ""
echo "To stop the runner:"
echo "  docker compose down"
echo ""
echo "To check runner status:"
echo "  docker compose ps"
