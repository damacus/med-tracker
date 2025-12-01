---
description: Review and address GitHub PR comments (especially Copilot)
---

# PR Review Workflow

Review and address comments on a GitHub Pull Request.

## Prerequisites

- PR number (e.g., 171)
- `gh` CLI authenticated: `gh auth login`

## Step 1: Fetch PR Review Comments

### Option A: MCP Tool (Recommended for Cascade)

```text
mcp5_pull_request_read with:
  owner: damacus
  repo: med-tracker
  pullNumber: <PR_NUMBER>
  method: get_review_comments
```

Returns structured JSON with:

- `body`: The comment text
- `path`: File path the comment is on
- `line`: Line number
- `user.login`: Who left the comment (e.g., "Copilot")
- `diff_hunk`: The code context

### Option B: `gh` CLI (Human-readable)

```fish
# Quick overview with all comments
gh pr view <PR_NUMBER> --comments

# Structured JSON for parsing
gh pr view <PR_NUMBER> --json title,body,comments,reviews,files | bat -p

# Filter to Copilot reviews only
gh pr view <PR_NUMBER> --json reviews --jq '.reviews[] | select(.author.login == "copilot") | .body'
```

## Step 2: Analyze Comments

For each comment:

1. **Understand the concern** - Read carefully, Copilot comments are often valid
2. **Verify against codebase** - Copilot can miss context that makes code correct
3. **Determine action**:
   - ✅ Code change required
   - ✅ Test addition needed
   - ✅ Documentation update
   - ❌ No action (explain why in response)

### Common Copilot Issues (Rails)

| Issue                      | Problem                                 | Fix                                       |
|----------------------------|-----------------------------------------|-------------------------------------------|
| **Enum comparisons**       | `person_type == 'adult'`                | Use `adult?` predicate or `:adult` symbol |
| **Association names**      | Wrong association in queries            | Verify names in model definition          |
| **Type mismatches**        | Comparing incompatible types            | Match types (symbol vs string)            |
| **Semantic changes**       | Modifying scopes affects usages         | Check all callers of changed code         |
| **Missing tests**          | New behavior untested                   | Add tests for specific behavior           |
| **Validation consistency** | Related validations use different logic | Ensure consistency across model           |
| **Unpersisted records**    | `exists?` misses built records          | Check both built and persisted            |

## Step 3: Make Changes

Follow TDD:

1. Write failing test for the issue (if applicable)
2. Implement the fix
3. Run tests: `task test`
4. Run linter: `rubocop -A`

## Step 4: Commit and Push

```fish
git add -A
git commit -m "fix: address review comments on PR #<NUMBER>"
git push
```

## Step 5: Reply to Each Comment Directly

Reply to each review comment individually using the comment ID from Step 1.

### Using MCP Tool (Reply to Review Comment)

```text
mcp5_add_comment_to_pending_review with:
  owner: damacus
  repo: med-tracker
  pullNumber: <PR_NUMBER>
  body: <response>
  path: <file_path from comment>
  line: <line number from comment>
  side: RIGHT
  subjectType: LINE
```

**Note**: This requires a pending review. If no pending review exists, create one first or reply via `gh` CLI.

### Using `gh` CLI (Simpler)

```fish
# Reply directly to a specific review comment (--silent suppresses output)
gh api repos/damacus/med-tracker/pulls/<PR_NUMBER>/comments/<COMMENT_ID>/replies \
  --silent \
  -f body="✅ Fixed in commit abc123 - [explanation]"
```

### Response Format

Keep replies concise and direct:

- ✅ **Fixed** in `<commit>` - [brief explanation]
- ⏭️ **Skipped** - [why no action needed]
- ❌ **Declined** - [why suggestion is incorrect, with reasoning]

## Tips

- **Don't blindly accept**: Copilot suggestions are often good but not always correct
- **Check context**: The comment may miss context that makes the code correct as-is
- **Add tests**: Even if the code is correct, adding tests proves it
- **Be thorough**: Address all comments, even if just to explain why no change is needed
- **Use `bat -p`**: Better formatting than `cat` for CLI output
