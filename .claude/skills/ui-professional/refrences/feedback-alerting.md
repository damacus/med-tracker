# Feedback & Alerting

[← Back to Main Skill](./SKILL.md)

## Purpose

This module governs all user feedback mechanisms: error messages, success confirmations, warnings, informational notices, banners, toasts, and inline validation. The goal is to ensure feedback reduces cognitive load, respects visual hierarchy, and guides users to solutions.

---

## Core Principles

### 1. Contextual Proximity

Feedback must appear **immediately adjacent** to the element or context that triggered it.

**Reasoning:** Users create mental models of cause-and-effect. When feedback appears far from the action, the brain must work to reconnect them, creating friction.

**Measurement:** Feedback should be within 200px or 1 visual grouping of the triggering element.

### 2. Visual Weight ↔ Severity Matching

The visual treatment must match the actual impact of the message.

**Reasoning:** Visual language sets expectations. Mismatched severity trains users to ignore important alerts or overreact to routine ones.

### 3. Signal Uniqueness

Each piece of feedback must serve a unique purpose. No message should appear in multiple locations simultaneously.

**Reasoning:** Redundancy lowers signal-to-noise ratio, making interfaces harder to scan and creating doubt ("Are these different errors?").

### 4. Actionable Content

Messages must tell users **what to do**, not just **what went wrong**.

**Reasoning:** State descriptions without guidance create frustration. Users need solutions, not status reports.

---

## Classification System

### Scope Dimension

| Scope | Definition | When to Use |
|-------|-----------|-------------|
| **Global** | Affects entire application/session | System down, maintenance mode, network disconnected, session expired |
| **Page** | Affects current page/view | Page load failed, permission denied for this page |
| **Section** | Affects a contained feature area | Form submission failed, data save error |
| **Field** | Affects single input/element | Validation error on specific field, invalid format |

### Severity Dimension

| Severity | Impact | User Action Required |
|----------|--------|---------------------|
| **Critical** | Blocks all interaction | Immediate action needed to continue using app |
| **High** | Blocks current task | Must fix to complete current workflow |
| **Medium** | Requires attention | Should address but not immediately blocking |
| **Low** | Informational | Optional awareness, no action needed |

---

## Pattern Library

### Pattern 1: Inline Field Validation

**When to use:** Single input has invalid value  
**Scope:** Field  
**Severity:** High (if blocking submission)

```html
<label for="email">Email</label>
<input 
  id="email" 
  type="email" 
  class="error"
  aria-invalid="true"
  aria-describedby="email-error"
>
<span id="email-error" class="error-message">
  Enter a valid email address (e.g., user@example.com)
</span>
```

**Visual treatment:**

- Red border on input
- Small text below input (not floating elsewhere)
- Icon (optional) next to text
- No global banner needed

**Copy guidelines:**

- ❌ "Invalid email"
- ✅ "Enter a valid email address (e.g., <user@example.com>)"

---

### Pattern 2: Form Section Alert

**When to use:** Multiple related fields have issues, or form submission failed  
**Scope:** Section  
**Severity:** High

```html
<form>
  <div class="alert alert-error" role="alert">
    <h3>Cannot submit form</h3>
    <p>Please fix the following errors:</p>
    <ul>
      <li><a href="#email">Email address is invalid</a></li>
      <li><a href="#password">Password must be at least 8 characters</a></li>
    </ul>
  </div>
  
  <!-- Form fields with individual inline errors -->
</form>
```

**Visual treatment:**

- Contained within form card/container
- Positioned at TOP of form (scanning pattern)
- Links to specific fields with errors
- Remains visible until issues resolved

**Important:** This replaces the need for a global banner. Don't use both.

---

### Pattern 3: Page-Level Banner

**When to use:** Page-specific issue that's not tied to a form  
**Scope:** Page  
**Severity:** Medium-High

```html
<main>
  <div class="banner banner-warning" role="alert">
    <p>This page is in read-only mode. <a href="/settings">Upgrade your plan</a> to edit.</p>
  </div>
  
  <!-- Page content -->
</main>
```

**Visual treatment:**

- Full-width within main content area (NOT fixed to viewport)
- Below page header, above primary content
- Can be dismissed if not critical
- Color matches severity (yellow for warning, red for error)

**Use cases:**

- Read-only mode for this page
- Preview/draft state
- Limited features on this page

---

### Pattern 4: Global Toast Notification

**When to use:** Background operation completed, or temporary informational message  
**Scope:** Global  
**Severity:** Low-Medium

```html
<div class="toast toast-success" role="status" aria-live="polite">
  <p>Settings saved successfully</p>
</div>
```

**Visual treatment:**

- Fixed position (usually top-right or bottom-center)
- Auto-dismisses after 3-5 seconds
- Small, non-intrusive
- Stacks if multiple appear

**Use cases:**

- "Saved successfully"
- "Copied to clipboard"
- "Email sent"
- Background task completed

**Critical rule:** Never use for errors that require user action. Toasts are for "FYI" messages only.

---

### Pattern 5: Global System Banner

**When to use:** System-wide state that affects all functionality  
**Scope:** Global  
**Severity:** Critical-High

```html
<div class="system-banner banner-critical" role="alert">
  <p>⚠️ Connection lost. Attempting to reconnect...</p>
</div>
```

**Visual treatment:**

- Fixed to top of viewport (overlays all content)
- Full-width
- High contrast (usually bright red or yellow)
- Remains until condition resolves

**Use cases:**

- Network disconnected
- Server maintenance mode
- Security breach notification
- Account suspended

**Critical rule:** This is the "nuclear option." Use sparingly. If you're considering this for a login prompt or form validation, **you're wrong**.

---

### Pattern 6: Modal Dialog

**When to use:** Requires immediate user decision before continuing  
**Scope:** Global (blocks interaction)  
**Severity:** Critical

```html
<dialog open role="alertdialog" aria-labelledby="dialog-title">
  <h2 id="dialog-title">Unsaved Changes</h2>
  <p>You have unsaved changes. Do you want to save before leaving?</p>
  <div class="dialog-actions">
    <button type="button" class="btn-secondary">Don't Save</button>
    <button type="button" class="btn-primary">Save</button>
  </div>
</dialog>
```

**Visual treatment:**

- Centers screen with backdrop
- Blocks all other interaction
- Must be dismissed to continue

**Use cases:**

- Destructive actions (delete confirmation)
- Unsaved changes warning
- Required decisions before proceeding

**Anti-pattern:** Don't use for routine notifications or non-blocking messages.

---

## Decision Tree

```
START: Need to show user feedback

1. Does this affect the ENTIRE APP/SYSTEM?
   YES → Use Global System Banner (Pattern 5) or Modal (Pattern 6)
   NO → Continue to 2

2. Is this about a SPECIFIC FORM/TASK?
   YES → Continue to 3
   NO → Use Page-Level Banner (Pattern 3) or Toast (Pattern 4)

3. Is this about ONE INPUT FIELD?
   YES → Use Inline Field Validation (Pattern 1)
   NO → Use Form Section Alert (Pattern 2)

4. FINAL CHECK: Am I using more than one pattern for the same message?
   YES → STOP. Choose the MOST SPECIFIC one only.
   NO → Proceed with implementation
```

---

## Validation Checklist

Before finalizing any feedback UI, verify:

- [ ] **Proximity:** Is feedback within 200px of triggering element?
- [ ] **Uniqueness:** Is this message appearing in only ONE location?
- [ ] **Scope Match:** Does visual weight (global banner, modal, etc.) match actual scope?
- [ ] **Actionability:** Does the message tell user HOW to fix the issue?
- [ ] **Accessibility:** Does it have proper ARIA labels and role?
- [ ] **Timing:** Does it appear immediately when triggered?
- [ ] **Persistence:** Does it remain visible until user can act on it?

---

## Common Mistakes & Fixes

### Mistake 1: The Double-Shout

**Problem:** Same error appears in global banner AND inline alert.

```html
<!-- ❌ WRONG -->
<div class="global-banner error">Please login to continue</div>
<form>
  <div class="alert error">Please login to continue</div>
</form>
```

**Fix:** Choose the most contextual location (inline alert) and remove global.

```html
<!-- ✅ CORRECT -->
<form>
  <div class="alert error" role="alert">
    <p>You must be logged in to submit this form.</p>
    <a href="/login" class="btn">Go to Login</a>
  </div>
</form>
```

---

### Mistake 2: The Floating Error

**Problem:** Validation error appears far from the input.

```html
<!-- ❌ WRONG -->
<div class="global-banner error">Email is invalid</div>
<form>
  <input type="email" id="email">
  <!-- Error is 400px away at top of page -->
</form>
```

**Fix:** Place error immediately below/beside the input.

```html
<!-- ✅ CORRECT -->
<form>
  <input type="email" id="email" aria-describedby="email-error" class="error">
  <span id="email-error" class="error-message">
    Enter a valid email (e.g., user@example.com)
  </span>
</form>
```

---

### Mistake 3: The Vague Alert

**Problem:** Message describes state but doesn't guide action.

```html
<!-- ❌ WRONG -->
<div class="alert error">Login failed</div>
```

**Fix:** Provide specific, actionable guidance.

```html
<!-- ✅ CORRECT -->
<div class="alert error" role="alert">
  <p>Login failed. Check your email and password, then try again.</p>
  <p><a href="/forgot-password">Forgot your password?</a></p>
</div>
```

---

### Mistake 4: The Wolf-Crying Banner

**Problem:** Using global system banner for routine state.

```html
<!-- ❌ WRONG -->
<div class="system-banner error">Error: Please login to continue</div>
```

**Fix:** This is not a system-wide emergency. Use local form alert.

```html
<!-- ✅ CORRECT -->
<form>
  <div class="alert info" role="alert">
    <p>Sign in to save your progress and access premium features.</p>
    <a href="/login" class="btn">Sign In</a>
  </div>
</form>
```

---

### Mistake 5: The Disappearing Error

**Problem:** Using toast for error that requires user action.

```html
<!-- ❌ WRONG -->
<div class="toast error">Payment failed</div>
<!-- Auto-dismisses before user can act -->
```

**Fix:** Use persistent alert that remains until addressed.

```html
<!-- ✅ CORRECT -->
<div class="alert error" role="alert">
  <h3>Payment could not be processed</h3>
  <p>Your card was declined. Please try a different payment method.</p>
  <button class="btn">Update Payment Method</button>
</div>
```

---

## Copy Guidelines

### Structure of Good Error Messages

1. **What happened** (brief)
2. **Why it happened** (if not obvious)
3. **What to do** (specific action)

**Example:**

```
❌ "Invalid input"
✅ "Email address is invalid. Use format: user@example.com"

❌ "Error 403"
✅ "You don't have permission to view this page. Contact your admin to request access."

❌ "Save failed"
✅ "Changes could not be saved. Check your internet connection and try again."
```

### Voice & Tone

- **Be direct, not apologetic:** "Enter a valid email" not "Sorry, it looks like maybe the email you entered might not be valid"
- **Be specific:** "Password must be at least 8 characters" not "Password is too short"
- **Be human:** "Check your internet connection" not "Network connectivity error detected"
- **Avoid jargon:** "This page is unavailable" not "HTTP 503 Service Unavailable"

---

## Accessibility Requirements

All feedback must:

- Use `role="alert"` for errors/warnings that need immediate attention
- Use `role="status"` for informational messages
- Include `aria-live="polite"` or `aria-live="assertive"` for dynamic content
- Link inputs to error messages with `aria-describedby`
- Mark invalid inputs with `aria-invalid="true"`
- Provide sufficient color contrast (WCAG AA minimum: 4.5:1)
- Not rely on color alone (use icons + text)

---

## Testing Your Feedback

Ask these questions:

1. **Proximity:** If I blur my eyes, is the error visually grouped with the cause?
2. **Redundancy:** Am I seeing the same message twice?
3. **Priority:** Does the visual treatment match how serious this actually is?
4. **Clarity:** Do I know exactly what to do to fix this?
5. **Timing:** Does this appear immediately when relevant?

---

## Related Patterns

- See [Buttons & Actions](./buttons-actions.md) for error recovery buttons
- See [Visual Hierarchy](./visual-hierarchy.md) for color and weight decisions
- See [Layout & Spacing](./layout-spacing.md) for positioning alerts in layouts

---

[← Back to Main Skill](./SKILL.md)
