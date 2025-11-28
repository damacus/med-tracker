---
description: Update dependencies
auto_execution_mode: 1
---

# Get Context

Read all dependency files e.g. package.json/Gemfile/github actions

For Ruby Gems run a bundle update. If this did not update anything consider removing the lockfile

# GitHub Actions

Lookup each dependency on github.
Look for the latest stable release

# Renovate/Mend

Look for any open PRs in the current repository
Implement those updates
