#!/usr/bin/env fish

set -l script_dir (status dirname)
set -l android_root (realpath "$script_dir/..")
set -l gradle_version ("$android_root/gradlew" --version --no-daemon | string collect)

echo "$gradle_version"
string match -rq 'Daemon JVM:.*17' "$gradle_version"
