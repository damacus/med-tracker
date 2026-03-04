---
name: github-pr
description: Automates the process of summarizing changes into a concise conventional commit and creating a GitHub Pull Request. Always uses a feature branch to ensure PR creation.
---

# GitHub PR Workflow

This skill automates the transition from local changes to a GitHub Pull Request by ensuring you're on a dedicated feature branch.

## Workflow

1.  **Summarize Changes**: Analyze the current `git diff` and provide a concise summary of the changes.
2.  **Conventional Commit**: Format the summary into a single, high-quality conventional commit message (e.g., `feat: add medication tracking`, `fix: resolve N+1 query in dashboard`).
3.  **Create Branch**:
    -   If on `main`, create a new branch based on the commit subject (e.g., `feat/add-med-tracking`): `git checkout -b <branch-name>`
    -   If already on a feature branch, verify it's the right one.
4.  **Commit**:
    -   Stage all relevant changes: `git add .`
    -   Commit with the generated message: `git commit -m "<message>"`
5.  **Push**: Push the current branch to the remote: `git push origin HEAD`
6.  **Create PR**: Use the `gh` CLI to create a pull request:
    ```bash
    gh pr create --title "<commit-subject>" --body "<commit-body>"
    ```

## Guidelines

-   **Concise Commits**: Focus on the "why" and "what," keeping the subject line under 50 characters.
-   **PR Body**: Include a brief description of the changes and link any relevant issues if mentioned in the context.
-   **Branch Naming**: Use `feat/`, `fix/`, `chore/`, or `refactor/` prefixes for branch names.
-   **Verification**: Ensure tests pass before initiating this workflow.
