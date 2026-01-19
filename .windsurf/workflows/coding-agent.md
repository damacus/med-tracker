---
description: Coding Agent
auto_execution_mode: 1
---

## YOUR ROLE - CODING AGENT

You are continuing work on a long-running autonomous development task.
This is a FRESH context window - you have no memory of previous sessions.

### STEP 1: GET YOUR BEARINGS (MANDATORY)

Start by orienting yourself:

```fish
# 1. See your working directory
pwd

# 2. List files to understand project structure
ls -la

# 3. Read the project specification to understand what you're building
cat docs/app_spec.txt

# 4. Check Beads for assigned work and roadmap
bd ready

# 5. Check recent git history
git log --oneline -20
```

Understanding the `docs/app_spec.txt` is critical - it contains the full requirements
for the application you're building.

### STEP 2: START TEST SERVERS (IF NOT RUNNING)

```fish
# Start test environment (Docker-based with PostgreSQL)
task test:up

# Or for local development with standalone PostgreSQL container
task local:db:up
```

### STEP 3: VERIFICATION TEST (CRITICAL!)

**MANDATORY BEFORE NEW WORK:**

The previous session may have introduced bugs. Before implementing anything
new, you MUST run verification tests.

Run 1-2 core functionality tests to verify the system is stable. Check `bd list` for tasks that were recently closed to identify regression risks.

**If you find ANY issues (functional or visual):**

- Update the corresponding issue in Beads using `bd update <issue_id> notes="..."`
- Add issues to a list
- Fix all issues BEFORE moving to new features
- This includes UI bugs like:
  - White-on-white text or poor contrast
  - Random characters displayed
  - Incorrect timestamps
  - Layout issues or overflow
  - Buttons too close together
  - Missing hover states
  - Console errors

### STEP 4: CHOOSE ONE TASK TO IMPLEMENT

Use `bd ready` to find the highest-priority task that is ready to be worked on.

Focus on completing one task perfectly and completing its testing steps in this session before moving on to other tasks.
It's ok if you only complete one task in this session, as there will be more sessions later that continue to make progress.

### STEP 5: IMPLEMENT THE FEATURE

Implement the chosen feature thoroughly:

1. Write the code (frontend and/or backend as needed)
2. Test manually using browser automation (see Step 6)
3. Fix any issues discovered
4. Verify the feature works end-to-end

### STEP 6: VERIFY WITH BROWSER AUTOMATION

**CRITICAL:** You MUST verify features through the actual UI.

Use browser automation tools:

- Navigate to the app in a real browser
- Interact like a human user (click, type, scroll)
- Take screenshots at each step
- Verify both functionality AND visual appearance

**DO:**

- Test through the UI with clicks and keyboard input
- Take screenshots to verify visual appearance
- Check for console errors in browser
- Verify complete user workflows end-to-end

**DON'T:**

- Only test with curl commands (backend testing alone is insufficient)
- Use JavaScript evaluation to bypass UI (no shortcuts)
- Skip visual verification
- Mark tests passing without thorough verification

### STEP 7: UPDATE ISSUE STATUS (MANDATORY)

After thorough verification, update the issue status using `bd update` or `bd close`.

```fish
# Claim the task
bd update <issue_id> status=in_progress

# Close the task after completion
bd close <issue_id>
```

**ONLY CLOSE THE ISSUE AFTER VERIFICATION WITH SCREENSHOTS AND TESTS PASSING.**

### STEP 8: COMMIT YOUR PROGRESS

Make a descriptive git commit:

```fish
git add .
git commit -m "Implement [feature name] - verified end-to-end

- Added [specific changes]
- Tested with browser automation
- Updated Beads: closed issue #X
- Screenshots in verification/ directory
"
```

### STEP 9: UPDATE PROGRESS NOTES

Update progress notes using `bd update` to add notes to relevant issues:

- What you accomplished this session
- Any issues discovered or fixed
- What should be worked on next
- Current status from `bd stats`

### STEP 10: END SESSION CLEANLY (LANDING THE PLANE)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Use `bd create` for anything that needs follow-up
2. **Run quality gates** (if code changed) - `task test`, `task rubocop`
3. **Update issue status** - Use `bd close` for finished work, `bd update` for in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:

   ```fish
   git pull --rebase
   bd sync
   git push
   git status  # MUST show "up to date with origin"
   ```

5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Update issue notes using `bd update`

**CRITICAL RULES:**

- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds

---

## AVAILABLE TASK COMMANDS

Run `task --list` to see all available commands. Key commands:

### Testing

```fish
# Run all tests in Docker (PostgreSQL)
task test

# Run specific test file
task test TEST_FILE=spec/models/user_spec.rb

# Rebuild test environment (drops database)
task test-rebuild

# Start/stop test server
task test:up
task test:stop

# View test logs
task test:logs
```

### Local Testing (faster, standalone PostgreSQL)

```fish
# Start local PostgreSQL container
task local:db:up

# Run non-browser tests locally
task local:test
task local:test TEST_FILE=spec/models/user_spec.rb

# Run browser tests locally (requires Playwright)
task local:test:browser

# Run all tests locally
task local:test:all

# Stop local database
task local:clean
```

### Development

```fish
# Start development server
task dev:up

# Seed development database
task dev:seed

# View logs / stop server
task dev:logs
task dev:stop

# Rebuild (drops database)
task dev:rebuild

# Open UI in browser
task dev:open-ui
```

### Beads (Issue Tracking)

```fish
# List all issues
bd list

# Find tasks ready to be worked on
bd ready

# Show details of a specific issue
bd show <issue_id>

# Update issue status/priority/assignee
bd update <issue_id> status=in_progress priority=1

# Close an issue
bd close <issue_id>

# Create a new issue
bd create "Title" description="..." issue_type=bug

# Get statistics
bd stats
```

### Linting

```fish
# Run RuboCop
task rubocop

# Run RuboCop with autocorrect
task rubocop AUTOCORRECT=true
```

---

## TESTING REQUIREMENTS

All testing must use browser automation tools.

Available tools:

- browser_navigate - Navigate to URL
- browser_snapshot - Capture accessibility snapshot (preferred over screenshot)
- browser_take_screenshot - Capture visual screenshot
- browser_click - Click elements by ref
- browser_type - Type text into elements
- browser_fill_form - Fill multiple form fields

Test like a human user with mouse and keyboard. Don't take shortcuts by using JavaScript evaluation.

---

## IMPORTANT REMINDERS

**Your Goal:** Production-quality application with all 200+ tests passing

**This Session's Goal:** Complete at least one feature perfectly

**Priority:** Fix broken tests before implementing new features

**Quality Bar:**

- Zero console errors
- Polished UI matching the design specified in app_spec.txt
- All features work end-to-end through the UI
- Fast, responsive, professional

**You have unlimited time.** Take as long as needed to get it right. The most important thing is that you
leave the code base in a clean state before terminating the session (Step 10).

---

Begin by running Step 1 (Get Your Bearings).
