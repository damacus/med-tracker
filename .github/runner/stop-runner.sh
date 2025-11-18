#!/bin/bash
set -e

# Script to stop the GitHub Actions self-hosted runner

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "Stopping GitHub Actions runner..."
docker compose down

echo "Runner stopped successfully!"
