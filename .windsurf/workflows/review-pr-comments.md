---
description: Review and address GitHub PR comments (especially Copilot)
---

# Review PR Comments Workflow

This workflow helps you quickly retrieve and address comments on GitHub pull requests, particularly from automated reviewers like Copilot.

## Steps

### 1. Fetch PR Overview

```bash
gh pr view <PR_NUMBER> --comments
```

This shows the PR title, description, and all comments in a readable format.

### 2. Get Structured PR Data (Optional)

```bash
gh pr view <PR_NUMBER> --json title,body,comments,reviews,files | bat -p
```

Use this to get JSON data for parsing with `jq` if needed.

### 3. Filter Copilot Reviews

```bash
gh pr view <PR_NUMBER> --json reviews --jq '.reviews[] | select(.author.login == "copilot") | .body' | bat -p
```

This extracts only Copilot's review comments.

### 4. Review Comments in GitHub UI

For detailed inline comments with code context, visit:

```text
https://github.com/<owner>/<repo>/pull/<PR_NUMBER>
```

### 5. Address Each Comment

For each comment:

- Read the affected file
- Understand the issue (common: enum comparisons, association names, type mismatches)
- Apply the suggested fix or implement a better solution
- Verify with tests

### 6. Run Tests

```bash
bundle exec rspec spec/models/ spec/controllers/
```

### 7. Commit Changes

```bash
git add .
git commit -m "fix: address copilot review comments"
git push
```

## Common Copilot Issues

### Enum Comparisons

**Problem**: Comparing enum with string (`person_type == 'adult'`)

**Fix**: Use predicate method (`person_type_adult?`) or symbol (`:adult`)

### Association Names

**Problem**: Using wrong association name in queries

**Fix**: Verify association names in model and use correct one

### Type Mismatches

**Problem**: Comparing incompatible types

**Fix**: Ensure types match (symbol vs string, integer vs string, etc.)

## Tips

- Use `bat -p` instead of `cat` for better formatting (plain output mode)
- The `gh` CLI requires authentication: `gh auth login`
- Copilot suggestions are usually correct but verify against domain logic
- Always run tests after making changes
