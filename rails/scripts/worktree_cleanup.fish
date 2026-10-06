#!/usr/bin/env fish

set -l worktrees (git worktree list --porcelain | string match -r "^worktree .*/\.worktrees/.*" | string replace -r "^worktree " "")

if test -z "$worktrees"
    echo "No worktrees found in .worktrees/"
    exit 0
end

echo "Checking worktrees in .worktrees/..."
set -l merged_wts
set -l active_wts

for wt in $worktrees
    # Get branch name for this specific worktree path
    set -l branch (git worktree list --porcelain | awk -v path="$wt" '$0 == "worktree "path {found=1; next} found && /^branch / {sub(/^branch refs\/heads\//, ""); print; exit} found && /^worktree / {exit}')
    
    if test -n "$branch"; and git branch --merged origin/main | grep -q "$branch\$"
        echo "  [MERGED] $wt ($branch)"
        set -a merged_wts $wt
    else if test -n "$branch"
        echo "  [ACTIVE] $wt ($branch)"
        set -a active_wts $wt
    else
        echo "  [DETACHED] $wt"
        set -a active_wts $wt
    end
end

if test -n "$merged_wts"
    echo
    set -l confirm (read -P "Do you want to forcibly remove the MERGED worktrees? (y/N): ")
    if test "$confirm" = "y"; or test "$confirm" = "Y"
        for wt in $merged_wts
            echo "Removing worktree: $wt"
            git worktree remove --force "$wt"
        end
        git worktree prune
    end
end

if test -n "$active_wts"
    echo
    set -l confirm_all (read -P "Do you want to forcibly remove ALL remaining worktrees in .worktrees/? (y/N): ")
    if test "$confirm_all" = "y"; or test "$confirm_all" = "Y"
        for wt in $active_wts
            echo "Removing worktree: $wt"
            git worktree remove --force "$wt"
        end
        git worktree prune
    end
end
