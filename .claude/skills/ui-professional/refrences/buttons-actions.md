# Buttons & Actions

[← Back to Main Skill](./SKILL.md)

## Purpose

This module governs all interactive action elements: buttons, links, CTAs, and clickable components. The goal is to create clear affordances, establish proper hierarchy among actions, and guide users toward successful task completion.

---

## Core Principles

### 1. Visual Affordance

Every interactive element must **look** interactive. Users should never wonder "Can I click this?"

**Reasoning:** Ambiguous affordance causes hesitation and exploration fatigue. Users shouldn't have to hover over everything to find interactions.

### 2. Action Hierarchy

Primary, secondary, and tertiary actions must have visually distinct treatments that match their importance.

**Reasoning:** When all buttons look equally important, users experience "choice paralysis" and miss the intended happy path.

### 3. State Communication

Every interactive element must have clear visual states: default, hover, active, focus, disabled, and loading.

**Reasoning:** State feedback confirms that the system received input and prevents double-submissions and user confusion.

### 4. Consequence Signaling

Destructive or irreversible actions must be visually distinct from constructive actions.

**Reasoning:** Users need warning signals before committing to actions they can't undo.

---

## Button Hierarchy

### Primary Actions

**Definition:** The main action the user should take on this screen/section. Usually only ONE per view.

**Visual treatment:**

- Solid fill with high contrast
- Largest size in the action group
- Positioned right-most in groups (Western reading pattern)
- Brand color or strong accent color

**Use for:**

- Form submissions ("Save", "Submit", "Continue")
- Primary conversions ("Buy Now", "Sign Up", "Subscribe")
- Completing workflows ("Finish", "Publish", "Send")

```html
<button type="submit" class="btn btn-primary">
  Save Changes
</button>
```

**Examples:**

- ✅ "Create Account" on signup page
- ✅ "Place Order" in checkout
- ✅ "Send Message" in compose view
- ❌ "Cancel" (this is secondary)
- ❌ Having 3 primary buttons on one screen

---

### Secondary Actions

**Definition:** Alternative actions that support the primary goal but are not the main path.

**Visual treatment:**

- Outline/ghost style or muted fill
- Same size as primary or slightly smaller
- Positioned left of primary button
- Neutral or muted colors

**Use for:**

- "Cancel" operations
- "Go Back" / "Previous"
- Alternative options ("Save as Draft" vs "Publish")
- Non-destructive alternatives

```html
<div class="button-group">
  <button type="button" class="btn btn-secondary">
    Cancel
  </button>
  <button type="submit" class="btn btn-primary">
    Save
  </button>
</div>
```

---

### Tertiary Actions

**Definition:** Low-priority actions that are available but shouldn't distract from primary/secondary actions.

**Visual treatment:**

- Text-only (link style) or minimal outline
- Smaller size
- Positioned away from primary actions or in overflow menus
- Low contrast

**Use for:**

- "Learn More" / "Help"
- "Skip this step"
- Settings or preferences
- Less common operations

```html
<button type="button" class="btn btn-tertiary">
  Skip for now
</button>
```

---

### Destructive Actions

**Definition:** Actions that delete, remove, or irreversibly change data.

**Visual treatment:**

- Red or warning color
- Often starts as secondary/outline style
- May require hover or second confirmation
- Clear warning icon

**Use for:**

- "Delete Account"
- "Remove Item"
- "Cancel Subscription"
- "Discard Changes"

```html
<button type="button" class="btn btn-destructive" data-confirm="true">
  Delete Project
</button>
```

**Critical rule:** Never make destructive actions the primary visual button unless the entire page/modal is about that destructive action.

---

## Button States

Every button must support these states:

### Default

How it appears before interaction.

```css
.btn-primary {
  background: #0066cc;
  color: white;
  cursor: pointer;
}
```

### Hover

Confirms the element is interactive.

```css
.btn-primary:hover {
  background: #0052a3;
  /* Slightly darker or lighter */
}
```

**Guideline:** 10-15% luminosity change is sufficient.

### Active/Pressed

Shows the button is being clicked.

```css
.btn-primary:active {
  background: #003d7a;
  transform: translateY(1px);
  /* Even darker, slight movement */
}
```

### Focus

Keyboard navigation indicator.

```css
.btn-primary:focus-visible {
  outline: 2px solid #0066cc;
  outline-offset: 2px;
}
```

**Critical rule:** Never remove focus states (`outline: none` without replacement). This breaks accessibility.

### Disabled

Indicates action is unavailable.

```css
.btn-primary:disabled {
  background: #cccccc;
  color: #666666;
  cursor: not-allowed;
  opacity: 0.6;
}
```

**Usage guideline:**

- Disable submit buttons until form is valid
- Disable action buttons during API calls
- Always include explanatory text near disabled buttons

```html
<!-- ❌ WRONG -->
<button disabled>Submit</button>

<!-- ✅ CORRECT -->
<button disabled>Submit</button>
<p class="help-text">Complete all required fields to submit</p>
```

### Loading

Shows asynchronous operation in progress.

```html
<button type="submit" class="btn btn-primary" disabled>
  <span class="spinner" aria-hidden="true"></span>
  <span>Saving...</span>
</button>
```

**Guidelines:**

- Disable button during loading
- Show spinner or loading indicator
- Change text to action in progress ("Saving...", "Processing...")
- Prevent double-submission

---

## Button Sizing & Spacing

### Minimum Touch Targets

**Mobile:** 44×44px minimum (Apple HIG, Material Design)  
**Desktop:** 32×32px minimum

**Reasoning:** Smaller targets increase error rate and frustration, especially on mobile devices.

### Spacing Between Buttons

**Horizontal groups:** 12-16px gap  
**Vertical stacks:** 8-12px gap

**Reasoning:** Sufficient spacing prevents mis-clicks and clearly separates actions.

### Text Padding

**Horizontal:** 16-24px  
**Vertical:** 8-12px

**Reasoning:** Creates visual breathing room and ensures text is legible.

---

## Button Placement Patterns

### Pattern 1: Form Actions (Bottom Right)

For forms and data entry, place actions bottom-right to follow Western reading pattern.

```html
<form>
  <!-- Form fields -->
  
  <div class="form-actions">
    <button type="button" class="btn btn-secondary">Cancel</button>
    <button type="submit" class="btn btn-primary">Save</button>
  </div>
</form>
```

**Order:** Cancel (left) → Primary Action (right)

---

### Pattern 2: Dialog Actions (Bottom Right or Bottom Center)

For modal dialogs, align actions based on content width.

```html
<dialog>
  <h2>Confirm Delete</h2>
  <p>This action cannot be undone.</p>
  
  <div class="dialog-actions">
    <button type="button" class="btn btn-secondary">Cancel</button>
    <button type="button" class="btn btn-destructive">Delete</button>
  </div>
</dialog>
```

**Order:** Safe action (left) → Destructive action (right)

---

### Pattern 3: Wizard Navigation (Split)

For multi-step processes, split navigation: Back (left), Next/Finish (right).

```html
<div class="wizard-nav">
  <button type="button" class="btn btn-secondary">
    ← Previous
  </button>
  
  <button type="button" class="btn btn-primary">
    Next →
  </button>
</div>
```

---

### Pattern 4: Toolbars (Grouped by Function)

For app toolbars, group related actions together.

```html
<div class="toolbar">
  <!-- Edit actions -->
  <div class="button-group">
    <button class="btn-icon" aria-label="Bold">B</button>
    <button class="btn-icon" aria-label="Italic">I</button>
    <button class="btn-icon" aria-label="Underline">U</button>
  </div>
  
  <!-- Alignment actions -->
  <div class="button-group">
    <button class="btn-icon" aria-label="Align left">⇐</button>
    <button class="btn-icon" aria-label="Align center">⇔</button>
    <button class="btn-icon" aria-label="Align right">⇒</button>
  </div>
</div>
```

**Principle:** Use visual dividers (borders, spacing) to group related actions.

---

## Button Labels

### Effective Label Principles

1. **Action-oriented verbs:** Start with a verb that describes what happens
2. **Specific outcomes:** Describe the result, not the mechanism
3. **Short but clear:** 1-3 words ideal, but clarity > brevity
4. **Contextual:** Should make sense without reading surrounding text

### Good vs. Bad Examples

| ❌ Bad | ✅ Good | Why |
|--------|---------|-----|
| "OK" | "Save Changes" | Specific outcome |
| "Click Here" | "Download Report" | Action-oriented |
| "Submit" | "Create Account" | Context-specific |
| "Yes" | "Delete Project" | Consequence-clear |
| "Proceed" | "Continue to Payment" | Next step explicit |

### Special Cases

**Loading states:**

- "Save" → "Saving..."
- "Send" → "Sending..."
- "Delete" → "Deleting..."

**Confirmation dialogs:**

- "Yes/No" → Repeat the action ("Delete", "Cancel")

**Multi-step processes:**

- Last step: "Next" → "Finish" or "Complete"

---

## Icon Usage

### When to Use Icons

**✅ Use icons when:**

- The icon is universally recognized (🔍 search, 🏠 home, ⚙️ settings)
- It's paired with text for clarity
- It's in a repeated pattern (toolbar, navigation)

**❌ Avoid icons when:**

- The meaning is ambiguous
- It's a primary action (use text)
- It's used alone for important actions

### Icon + Text Patterns

```html
<!-- Icon before text (most common) -->
<button class="btn btn-primary">
  <svg class="icon" aria-hidden="true"><!-- icon --></svg>
  <span>Save Changes</span>
</button>

<!-- Icon after text (directional) -->
<button class="btn btn-secondary">
  <span>Next</span>
  <svg class="icon" aria-hidden="true">→</svg>
</button>

<!-- Icon only (with accessible label) -->
<button class="btn-icon" aria-label="Close dialog">
  <svg aria-hidden="true">×</svg>
</button>
```

**Accessibility:** Always use `aria-label` or `aria-labelledby` when icon is the only visible content.

---

## Common Mistakes & Fixes

### Mistake 1: Too Many Primary Buttons

**Problem:** Multiple buttons compete for attention.

```html
<!-- ❌ WRONG -->
<div class="actions">
  <button class="btn btn-primary">Save</button>
  <button class="btn btn-primary">Save & Continue</button>
  <button class="btn btn-primary">Cancel</button>
</div>
```

**Fix:** Use hierarchy to guide user to preferred action.

```html
<!-- ✅ CORRECT -->
<div class="actions">
  <button class="btn btn-secondary">Cancel</button>
  <button class="btn btn-secondary">Save</button>
  <button class="btn btn-primary">Save & Continue</button>
</div>
```

---

### Mistake 2: Destructive Action as Primary

**Problem:** Delete/destructive button styled as primary action.

```html
<!-- ❌ WRONG -->
<button class="btn btn-primary">Delete Account</button>
```

**Fix:** Use destructive styling and make it secondary unless entire context is about deletion.

```html
<!-- ✅ CORRECT -->
<div class="actions">
  <button class="btn btn-secondary">Cancel</button>
  <button class="btn btn-destructive">Delete Account</button>
</div>
```

---

### Mistake 3: No Visual Feedback

**Problem:** Button doesn't show hover/active states.

```css
/* ❌ WRONG */
.btn {
  background: blue;
  /* No hover or active states */
}
```

**Fix:** Implement all interactive states.

```css
/* ✅ CORRECT */
.btn {
  background: blue;
  transition: background 0.2s;
}
.btn:hover {
  background: darkblue;
}
.btn:active {
  background: navy;
}
.btn:disabled {
  background: gray;
  cursor: not-allowed;
}
```

---

### Mistake 4: Disabled Without Context

**Problem:** Disabled button with no explanation.

```html
<!-- ❌ WRONG -->
<button type="submit" disabled>Submit</button>
```

**Fix:** Provide clear guidance on why it's disabled.

```html
<!-- ✅ CORRECT -->
<button type="submit" disabled>Submit</button>
<p class="help-text" role="status">
  Complete all required fields to enable submission
</p>
```

---

### Mistake 5: Generic Labels

**Problem:** Vague or context-free button text.

```html
<!-- ❌ WRONG -->
<button>OK</button>
<button>Submit</button>
```

**Fix:** Use specific, action-oriented labels.

```html
<!-- ✅ CORRECT -->
<button>Save Settings</button>
<button>Create New Project</button>
```

---

## Accessibility Requirements

All buttons must:

- Use semantic `<button>` or `<a>` elements (not `<div>` with click handlers)
- Have visible and accessible text (use `aria-label` for icon-only buttons)
- Support keyboard interaction (Space/Enter for buttons, Enter for links)
- Have visible focus indicators (`:focus-visible`)
- Use `type="button"` for non-submit buttons in forms
- Use `aria-pressed="true/false"` for toggle buttons
- Use `aria-expanded="true/false"` for buttons that show/hide content
- Have sufficient color contrast (WCAG AA: 4.5:1 minimum)

---

## Testing Your Buttons

Ask these questions:

1. **Affordance:** Can I tell this is clickable without hovering?
2. **Hierarchy:** Is the most important action the most prominent?
3. **States:** Do hover, focus, active, and disabled states exist and work?
4. **Label:** Does the button text describe what will happen?
5. **Safety:** Are destructive actions visually distinct and confirmed?
6. **Keyboard:** Can I reach and activate this with keyboard only?
7. **Touch:** Is the touch target at least 44×44px on mobile?

---

## Decision Tree

```
START: Need to add a button/action

1. What is the action's priority in this context?
   - Main goal → Primary button (solid, accent color)
   - Alternative/cancel → Secondary button (outline/ghost)
   - Low priority → Tertiary button (text/link style)
   - Destructive → Destructive button (red/warning)

2. Where should it be positioned?
   - Form action → Bottom right of form
   - Dialog action → Bottom right/center of dialog
   - Navigation → Split left/right
   - Toolbar → Group with related actions

3. Does it need an icon?
   - Universally recognized → Yes, with text
   - Ambiguous → No icon, text only
   - Space constrained → Icon only with aria-label

4. What states are needed?
   - Always → Default, hover, active, focus, disabled
   - Async operation → Add loading state
   - Toggle → Add pressed/unpressed states

5. What's the label?
   - Start with verb → Describes action
   - Specific → "Save Changes" not "OK"
   - Contextual → Makes sense alone
```

---

## Related Patterns

- See [Feedback & Alerting](./feedback-alerting.md) for confirmation messages after actions
- See [Visual Hierarchy](./visual-hierarchy.md) for color and emphasis decisions
- See [Layout & Spacing](./layout-spacing.md) for button group positioning

---

[← Back to Main Skill](./SKILL.md)
