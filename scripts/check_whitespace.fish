#!/usr/bin/env fish
if test (count $argv) -gt 1
    echo 'Usage: check_whitespace.fish [base-ref]' >&2
    exit 64
end

set base HEAD
if set -q argv[1]
    set base $argv[1]
end
set root (git rev-parse --show-toplevel)
or exit $status
cd "$root"
or exit $status
set comparison (git merge-base HEAD "$base")
or exit $status
set original_index (git rev-parse --path-format=absolute --git-path index)
or exit $status
set original_objects (git rev-parse --path-format=absolute --git-path objects)
or exit $status
set -g whitespace_directory (mktemp -d)
or exit $status

function cleanup_whitespace --on-event fish_exit
    command rm -rf -- "$whitespace_directory"
end
function interrupt_whitespace --on-signal INT
    set -g whitespace_interrupted 130
end
function terminate_whitespace --on-signal TERM
    set -g whitespace_interrupted 143
end

set -gx GIT_INDEX_FILE "$whitespace_directory/index"
set -gx GIT_OBJECT_DIRECTORY "$whitespace_directory/objects"
command mkdir -- "$GIT_OBJECT_DIRECTORY"
or exit $status
set -l quoted_objects (string replace -a '\\' '\\\\' -- "$original_objects" | string replace -a '"' '\\"')
if set -q GIT_ALTERNATE_OBJECT_DIRECTORIES; and test -n "$GIT_ALTERNATE_OBJECT_DIRECTORIES"
    set -gx GIT_ALTERNATE_OBJECT_DIRECTORIES "\"$quoted_objects\":$GIT_ALTERNATE_OBJECT_DIRECTORIES"
else
    set -gx GIT_ALTERNATE_OBJECT_DIRECTORIES "\"$quoted_objects\""
end
if test -f "$original_index"
    command cp -p -- "$original_index" "$GIT_INDEX_FILE"
    or exit $status
else
    git read-tree HEAD
    or exit $status
end

git diff --cached --check "$comparison" --
or exit $status
git -c core.splitIndex=false add -A -- .
or exit $status
if set -q whitespace_interrupted
    exit $whitespace_interrupted
end
git diff --cached --check "$comparison" --
set check_status $status
if set -q whitespace_interrupted
    exit $whitespace_interrupted
end
exit $check_status
