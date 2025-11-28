#!/bin/bash
set -e

# If running as root, fix permissions and drop privileges
if [ "$(id -u)" = "0" ]; then
    echo "Running as root. Fixing permissions..."

    # Fix permissions for runner work directory
    if [ -d "/tmp/runner" ]; then
        echo "Fixing permissions for /tmp/runner..."
        chown -R runner:runner /tmp/runner
    fi

    # Allow runner to access docker socket by group membership
    if [ -e "/var/run/docker.sock" ]; then
        SOCKET_GID=$(stat -c '%g' /var/run/docker.sock)
        echo "Docker socket GID: $SOCKET_GID"

        if getent group "$SOCKET_GID" >/dev/null; then
            GROUP_NAME=$(getent group "$SOCKET_GID" | cut -d: -f1)
            echo "Adding runner to existing group $GROUP_NAME ($SOCKET_GID)..."
            usermod -aG "$GROUP_NAME" runner
        else
            echo "Creating group docker-sock with GID $SOCKET_GID..."
            groupadd -g "$SOCKET_GID" docker-sock
            usermod -aG docker-sock runner
        fi
    fi

    echo "Dropping privileges to runner user..."
    exec gosu runner "$0" "$@"
fi

# Check required environment variables
if [ -z "$GITHUB_TOKEN" ]; then
    echo "Error: GITHUB_TOKEN environment variable is required"
    exit 1
fi

if [ -z "$GITHUB_REPOSITORY" ]; then
    echo "Error: GITHUB_REPOSITORY environment variable is required"
    exit 1
fi

RUNNER_NAME="${RUNNER_NAME:-med-tracker-runner}-${HOSTNAME}"
RUNNER_LABELS="${RUNNER_LABELS:-self-hosted,linux,x64,med-tracker}"

# Get registration token
echo "Getting registration token..."
REGISTRATION_TOKEN=$(curl -sX POST \
    -H "Authorization: token ${GITHUB_TOKEN}" \
    -H "Accept: application/vnd.github.v3+json" \
    "https://api.github.com/repos/${GITHUB_REPOSITORY}/actions/runners/registration-token" | jq -r .token)

if [ -z "$REGISTRATION_TOKEN" ] || [ "$REGISTRATION_TOKEN" = "null" ]; then
    echo "Error: Failed to get registration token. Check your GITHUB_TOKEN permissions."
    exit 1
fi

# Configure runner
echo "Configuring runner..."
./config.sh \
    --url "https://github.com/${GITHUB_REPOSITORY}" \
    --token "${REGISTRATION_TOKEN}" \
    --name "${RUNNER_NAME}" \
    --labels "${RUNNER_LABELS}" \
    --work "_work" \
    --unattended \
    --replace

# Cleanup function
cleanup() {
    echo "Caught signal, performing cleanup..."

    # Stop the runner service first if it's running
    if [ -n "$runner_pid" ]; then
        echo "Stopping runner process $runner_pid..."
        kill -TERM "$runner_pid" 2>/dev/null || true
        wait "$runner_pid" 2>/dev/null || true
    fi

    echo "Removing runner..."
    REMOVE_TOKEN=$(curl -sX POST \
        -H "Authorization: token ${GITHUB_TOKEN}" \
        -H "Accept: application/vnd.github.v3+json" \
        "https://api.github.com/repos/${GITHUB_REPOSITORY}/actions/runners/remove-token" | jq -r .token)

    if [ -n "$REMOVE_TOKEN" ] && [ "$REMOVE_TOKEN" != "null" ]; then
        ./config.sh remove --token "${REMOVE_TOKEN}"
    else
        echo "Error: Failed to get remove token."
    fi
}

trap cleanup SIGINT SIGTERM EXIT

# Run the runner
echo "Starting runner..."
./run.sh &
runner_pid=$!

wait "$runner_pid"
