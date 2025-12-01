---
description: Review and address GitHub PR comments (especially Copilot)
---

# PR Review Workflow

Review and address comments on a GitHub Pull Request.

## Prerequisites

- PR number (e.g., 171)

## Step 1: Fetch PR Review Comments

Use the GitHub MCP tool to get review comments:

```text
mcp5_pull_request_read with:
  owner: damacus
  repo: med-tracker
  pullNumber: <PR_NUMBER>
  method: get_review_comments
```

This returns structured JSON with:

- `body`: The comment text
- `path`: File path the comment is on
- `line`: Line number
- `user.login`: Who left the comment (e.g., "Copilot")
- `diff_hunk`: The code context

Alternative using `gh` CLI (non-interactive):

```fish
gh pr view <PR_NUMBER> --json reviews,comments --jq '.reviews[].body, .comments[].body'
```

## Step 2: Analyze Comments

For each comment:

1. **Understand the concern** - Read carefully, Copilot comments are often valid
2. **Check if it's correct** - Copilot can make mistakes, verify against codebase
3. **Determine action needed**:
   - Code change required
   - Test addition needed
   - Documentation update
   - No action (explain why in response)

### Common Copilot Concerns

- **Semantic changes**: When modifying associations/scopes, check all usages
- **Missing tests**: Add tests that verify the specific behavior mentioned
- **Validation consistency**: Ensure related validations use consistent logic
- **Breaking changes**: Document and test edge cases

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

## Step 5: Reply to Comments

Use the GitHub MCP tool to add a comment:

```text
mcp5_add_issue_comment with:
  owner: damacus
  repo: med-tracker
  issue_number: <PR_NUMBER>
  body: <response explaining what was fixed>
```

Format response as:

```markdown
## Addressed Review Comments

### Comment 1: [Brief description]
✅ **Fixed** - [What was done]

### Comment 2: [Brief description]
✅ **Fixed** - [What was done]

All tests pass.
```

## Tips

- **Don't blindly accept**: Copilot suggestions are often good but not always correct
- **Check context**: The comment may miss context that makes the code correct as-is
- **Add tests**: Even if the code is correct, adding tests proves it
- **Be thorough**: Address all comments, even if just to explain why no change is needed
