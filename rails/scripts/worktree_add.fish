#!/usr/bin/env fish

set -l agent (read -P "Agent (codex/gemini/claude): ")
set -l issue (read -P "Issue number: ")

if test -z "$agent"; or test -z "$issue"
    echo "Error: Agent and Issue number are required."
    exit 1
end

if test -d .worktrees/issue-$issue
    echo "Error: .worktrees/issue-$issue already exists."
    exit 1
end

git worktree add .worktrees/issue-$issue -b $agent/$issue origin/main
