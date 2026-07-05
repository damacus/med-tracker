# MedTracker Agent Guide

> Keep `AGENTS.md` and `agents.md` in sync. `CLAUDE.md` is a separate Claude Code guide.

## Mandatory First Steps

- **Serena MCP** — For coding, review, or architecture tasks, use tool discovery for `serena initial_instructions` if Serena tools are not already visible, then call Serena `initial_instructions` before broad code exploration or implementation. Activate the project if needed. Prefer Serena symbolic navigation for code structure; if Serena is unavailable or lacks the needed tool, say so briefly and continue with `rg`, `sed`, and normal repo tools.
- **Ruby skill** — For Ruby or Rails coding, review, or debugging tasks, load the Ruby skill before implementation and follow the applicable reference files.
- **Context7** — Fetch current documentation with Context7 before answering or implementing library, framework, SDK, API, CLI, or cloud-service usage details.

## TDD

Follow Red-Green-Refactor — no production code without a failing test first.

1. **Red** — write a test that fails
2. **Green** — write the minimum code to make it pass
3. **Refactor** — clean up while keeping tests green

## Non-obvious rules

- **Shell** — Fish syntax only (`set VAR value`, `(cmd)`, `if … end`)
- **Comments** — Never add or remove comments unless explicitly asked
- **Person enum** — `adult:0`, `minor:1`, `dependent_adult:2`; minors/dependent_adults always `has_capacity:false`
- **PostgreSQL 18** — Use version 18, not 17, in all configs and docs
- **Fixture password** — All dev/test fixture users have password `password`
- **JSON inspection** — Use `jq` for JSON search/filtering; do not write Ruby/Python scripts for ad hoc JSON parsing

## Stack

- Ruby 3.4
- Rails 8.1
- PostgreSQL 18
- RSpec, Capybara, VCR, Rails fixtures
- RubyUI, Phlex, Hotwire, Propshaft
- RuboCop

## Code and Architecture

- Use RuboCop as the source of truth for Ruby style.
- Prefer clear names, small private methods, guard clauses, and Enumerable methods where they improve readability.
- Keep controllers focused on HTTP concerns; put business logic in models, POROs, or service objects.
- Use Phlex components in `app/components/`.
- Add nil-safety guards in policy/component code when records or associations may be absent.
- Use `respond_to?` guards where a policy record may be a Class, such as `new` actions.

## Views and UI

- All views are Ruby/Phlex files. Do not create `.erb` files.
- Use RubyUI components when an equivalent component exists, especially for headings, text, links, buttons, forms, cards, tables, dialogs, badges, avatars, separators, popovers, tooltips, and calendar/date inputs.
- Keep UI accessible: keyboard access, labels, alt text, visible focus states, WCAG AA text contrast, descriptive link text, associated error messages, table headers, dismissible focus-trapping dialogs, and logical heading hierarchy.
- For UI work, verify through the actual UI with browser automation and capture desktop/mobile screenshots when the change is visible.
- Fetch current RubyUI docs with Context7 when component API details are unclear.

## Data Access and Performance

- Do not execute database queries inside view components or loops.
- Eager-load associations used by views with `includes`, `preload`, or `eager_load` in the controller/query boundary.
- Pass preloaded associations or query results into components instead of re-querying in nested methods.
- Use `size` on loaded collections instead of `count`.
- Use database filtering/sorting (`where`, `order`) for unloaded ActiveRecord relations.
- Use in-memory filtering (`select`, `reject`) only on already-loaded/eager-loaded collections.
- Prefer `exists?` over `find_by` when only checking existence.
- Wrap related writes in `ActiveRecord::Base.transaction` and use bang persistence methods inside transactions.

## Commands

Use `task` for everything. Never run `docker compose`, `bin/dev`, or `bundle exec rspec` directly.

> **Note**: For most `task dev:*` commands, an equivalent `task test:*` command exists (e.g., `task test:up`, `task test:port`).

| What | Command |
|---|---|
| Run tests | `task test` |
| Lint | `task rubocop` |
| Start dev server | `task dev:up` |
| Build dev images | `task dev:build` |
| View dev logs | `task dev:logs` |
| Stop dev server | `task dev:stop` |
| Get dev port | `task dev:port` |
| Open in browser | `task dev:open-ui` |
| Seed database | `task dev:seed` |
| Migrate | `task dev:db-migrate` |
| Rebuild (destructive) | `task dev:rebuild` |
| Run Brakeman | `task brakeman` |
| Run RuboCop autocorrect | `task rubocop AUTOCORRECT=true` |
| Stop everything | `task stop-all` |
| Run local Playwright browser tests | `task playwright` |
| List all tasks | `task -l` |

## Docker Development

- Development uses a bind mount, so Ruby, config, lib, spec, and database file changes sync into the container automatically.
- Rebuild after changing `Gemfile`, `Gemfile.lock`, `package.json`, `yarn.lock`, or Docker configuration.
- Use `task dev:db-migrate` after migrations.
- Use `task dev:rebuild` only for a destructive fresh start.
- Do not use Docker Compose watch; the bind mount and Rails reloader already provide live updates.

## Testing

- Write RSpec tests in `_spec.rb` files using Rails/RSpec conventions.
- Test public APIs and observable behavior, not implementation details.
- Use Rails fixtures in `spec/fixtures/`; keep fixture relationships realistic and avoid duplicate unique attributes.
- Use VCR cassettes in `spec/vcr_cassettes/` for external API mocking.
- Policy changes need explicit coverage for relevant roles: admin, clinician, self, carer, parent, and unauthorized users.
- New model validations need positive and negative test cases.
- Admin CRUD flows need success, validation error, duplicate handling, and immediate-usability coverage.
- Capybara tests should use exact labels/text from the views. Prefer `click_link` for links and `click_button` for buttons.
- Specs that use browser features need `:browser` or `type: :system`.

## Screenshots for PRs

```fish
task dev:up
task dev:port          # → e.g. 3000
# then use the playwright-cli skill to navigate and screenshot
```

Save PR screenshots under `docs/screenshots/` with page and viewport in the filename, for example `dashboard-desktop.png` and `dashboard-mobile.png`.

## Quality gates (run before every push)

```fish
task rubocop          # lint — must pass with no offenses
task test             # full test suite in Docker — must be green
```

Run a single file during development:

```fish
task test TEST_FILE=spec/path/to/file_spec.rb
```

Never push if either command fails.

## Review and Security

- Code review findings should prioritize correctness, missing coverage, authorization gaps, N+1 queries, nil safety, race conditions, unsafe SQL, mass assignment, and existing pattern violations.
- PR review comments must be checked against the current code before changing anything; do not blindly apply Copilot or bot suggestions.
- Address each actionable PR review comment directly after pushing the fix or explain why no code change was needed.
- Security review should use `task brakeman` and manual review of authentication, authorization, strong parameters, model validations, raw SQL, secret handling, security headers, dependency risk, audit trails, and medication/health-data access controls.
- Document false positives or accepted risks before ignoring security findings.

## PR and Commit Text

- Use Conventional Commits with scopes when useful, for example `fix(medicines): handle missing reorder threshold`.
- Keep changes atomic and focused.
- For PR titles and squash messages, use the same Conventional Commit style.
- PR summaries should be human-readable first: explain what problem the change solves, why the change exists, and what future bugs or confusion it prevents. Write for someone passing by the PR who does not already know the implementation details.
- For refactors or infrastructure work, describe the before/after contract in plain language, for example: "this used to be handled differently in several places; now one shared rule handles it consistently."
- Do not include routine test sections in PR descriptions; CI is the source of truth. Mention verification only when it is manual, unusual, blocked, or not covered by CI.
- Include screenshots for visible UI changes.

## Session close (mandatory)

```fish
git pull --rebase
git push
```

Work is not done until `git push` succeeds.

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```fish
   git pull --rebase
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
