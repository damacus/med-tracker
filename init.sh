#!/usr/bin/env bash
# MedTracker Development Environment Setup
# This script initializes the development environment for MedTracker

set -e

echo "=========================================="
echo "  MedTracker Development Environment"
echo "=========================================="
echo ""

# Check for required tools
check_tool() {
    if ! command -v "$1" &> /dev/null; then
        echo "❌ $1 is required but not installed."
        return 1
    else
        echo "✅ $1 found"
        return 0
    fi
}

echo "Checking required tools..."
check_tool "docker" || exit 1
check_tool "task" || { echo "Install task from https://taskfile.dev"; exit 1; }

echo ""
echo "Starting development environment..."
echo ""

# Start the development server
task dev-up

echo ""
echo "Waiting for services to be ready..."
sleep 5

# Seed the database with fixtures
echo "Seeding database with test data..."
task dev-seed

echo ""
echo "=========================================="
echo "  Development Environment Ready!"
echo "=========================================="
echo ""
echo "Access the application at: http://localhost:3000"
echo ""
echo "Test Users (password: 'password'):"
echo "  - damacus@example.com (administrator)"
echo "  - dr.jones@example.com (doctor)"
echo "  - nurse.smith@example.com (nurse)"
echo "  - bob.smith@example.com (carer)"
echo "  - jane.doe@example.com (parent)"
echo ""
echo "Useful commands:"
echo "  task dev-logs    - View server logs"
echo "  task dev-stop    - Stop the server"
echo "  task test        - Run tests"
echo "  task rubocop     - Run linter"
echo ""
echo "To view all available commands: task --list"
echo ""
