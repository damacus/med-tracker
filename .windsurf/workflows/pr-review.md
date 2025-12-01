---
description: Review and address PR comments (especially Copilot)
---

# PR Review Workflow

Review pull request comments and address feedback from automated reviewers like GitHub Copilot.

## Prerequisites

- PR number (e.g., `170`)
- GitHub MCP server connected OR `gh` CLI authenticated

## Step 1: Retrieve PR Comments

### Option A: GitHub MCP (Preferred)

Use the GitHub MCP tools to fetch PR details and comments:

```text
mcp5_pull_request_read(owner: "damacus", repo: "med-tracker", pullNumber: <PR_NUMBER>, method: "get")
mcp5_pull_request_read(owner: "damacus", repo: "med-tracker", pullNumber: <PR_NUMBER>, method: "get_review_comments")
mcp5_pull_request_read(owner: "damacus", repo: "med-tracker", pullNumber: <PR_NUMBER>, method: "get_reviews")
```

### Option B: gh CLI

```fish
# Get PR details with all comments (no pagination)
gh pr view <PR_NUMBER> --json title,body,comments,reviews,files | jq '.'

# Filter only Copilot review comments
gh api repos/damacus/med-tracker/pulls/<PR_NUMBER>/comments --paginate | \
  jq '[.[] | select(.user.login == "Copilot") | {path, line, body}]'

# Get review summary
gh pr view <PR_NUMBER> --json reviews --jq '.reviews[] | select(.author.login | test("copilot"; "i")) | .body'
```

## Step 2: Analyze Comments

For each comment, determine:

| Field | Question |
|-------|----------|
| **Valid?** | Is the feedback technically correct? |
| **Applicable?** | Does it apply to this project's conventions? |
| **Priority** | Critical (blocks merge), Important (should fix), Minor (nice to have) |
| **Action** | Fix, Acknowledge, Dismiss with reason |

### Common Copilot False Positives

1. **FactoryBot vs Fixtures**: Copilot may suggest fixtures when factories are intentionally used for dynamic data
2. **Private method testing**: Sometimes testing private methods is acceptable for complex internal logic
3. **Missing tests**: May suggest tests that already exist elsewhere
4. **Style preferences**: May conflict with project-specific conventions

## Step 3: Address Comments

### For Valid Comments

1. Read the relevant file(s) to understand context
2. Make the fix following TDD (write failing test first if applicable)
3. Run tests: `task test TEST_FILE=<spec_file>`
4. Commit with conventional format: `fix: address PR review feedback`

### For Invalid/Inapplicable Comments

Reply to the comment explaining why it doesn't apply:

```text
mcp5_add_issue_comment(owner: "damacus", repo: "med-tracker", issue_number: <PR_NUMBER>, body: "...")
```

## Step 4: Verify All Tests Pass

```fish
// turbo
task test
```

## Step 5: Push Changes

```fish
git add -A
git commit -m "fix: address PR review feedback"
git push
```

## Example: PR #170 Copilot Comments

| # | Comment | Valid? | Action |
|---|---------|--------|--------|
| 1 | Use fixtures instead of FactoryBot | ✅ | Replace `create(:person_medicine)` with fixture |
| 2 | Add tests for update/destroy | ✅ | Add `trace_update` and `trace_destroy` tests |
| 3 | Add test for prescription source | ✅ | Add test with prescription fixture |
| 4 | Don't test private methods | ✅ | Refactor to test via span attributes |
| 5 | Clean up span processor | ✅ | Add before/after hooks for cleanup |

## Tips

- **Batch related fixes**: Group similar changes into one commit
- **Don't over-engineer**: Address the specific feedback, don't refactor unrelated code
- **Test incrementally**: Run tests after each significant change
- **Document decisions**: If dismissing feedback, explain why in the PR
