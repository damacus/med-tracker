#!/usr/bin/env fish

set test_file $argv[1]

if not docker info >/dev/null 2>&1
    echo 'Docker is unavailable. Start Docker, then rerun task test:preflight.' >&2
    exit 10
end

if not docker image inspect med-tracker-web-test >/dev/null 2>&1
    echo 'Test image med-tracker-web-test is missing. Run task test:build, then rerun task test:preflight.' >&2
    exit 11
end

if not task test TEST_FILE=$test_file
    echo "Test preflight spec failed: $test_file" >&2
    exit 12
end

echo "Test preflight passed: $test_file"
