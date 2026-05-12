# Visual Hierarchy

[← Back to Main Skill](./SKILL.md)

## Purpose

This module governs how to establish importance, guide user attention, and create a clear reading order through visual treatment. The goal is to ensure users can quickly identify what matters most and navigate content efficiently.

---

## Core Principles

### 1. Visual Weight = Functional Importance

Elements should look as important as they actually are. No more, no less.

**Reasoning:** When visual weight doesn't match functional importance, users are misled about what to focus on or what to do next.

**Anti-pattern:** Making a "Skip" link look more prominent than a "Sign Up" button.

### 2. Progressive Disclosure

Show the most important information first; reveal details on demand.

**Reasoning:** Users scan pages in predictable patterns (F-pattern, Z-pattern). Hierarchy should match these patterns.

### 3. Limited Emphasis

If everything is emphasized, nothing is. Use a maximum of 2-3 hierarchy levels per section.

**Reasoning:** Too many competing elements creates visual chaos and decision paralysis.

### 4. Consistent Hierarchy System

Use the same visual treatments for the same levels of importance across the interface.

**Reasoning:** Consistency builds a learnable interface. Users should recognize patterns.

---

## Hierarchy Tools

### Size

Larger elements draw more attention.

**Text sizing scale (example):**

- **Hero headline:** 48-72px (main landing page title)
- **H1:** 32-40px (page title)
- **H2:** 24-28px (section heading)
- **H3:** 20-24px (subsection heading)
- **Body:** 16-18px (main content)
- **Small:** 14px (captions, metadata)
- **Tiny:** 12px (legal, footnotes)

**Principle:** Each level should be noticeably different (at least 20% size difference).

```html
<!-- ✅ CORRECT - Clear hierarchy -->
<h1 style="font-size: 36px;">Page Title</h1>
<h2 style="font-size: 24px;">Section Heading</h2>
<p style="font-size: 16px;">Body content</p>

<!-- ❌ WRONG - Too similar -->
<h1 style="font-size: 20px;">Page Title</h1>
<h2 style="font-size: 18px;">Section Heading</h2>
<p style="font-size: 16px;">Body content</p>
```

---

### Weight (Font Weight)

Heavier fonts command more attention.

**Weight scale:**

- **300 (Light):** De-emphasized text (captions)
- **400 (Regular):** Body text
- **500 (Medium):** Slight emphasis (labels)
- **600 (Semibold):** Headings, important labels
- **700 (Bold):** Strong emphasis, primary headings
- **900 (Black):** Rare, extreme emphasis

**Guideline:** Don't rely solely on weight. Combine with size or color.

```html
<!-- ✅ CORRECT - Weight supports hierarchy -->
<h2 style="font-weight: 600; font-size: 24px;">Important Section</h2>
<p style="font-weight: 400; font-size: 16px;">Regular content</p>

<!-- ❌ WRONG - Weight alone insufficient -->
<span style="font-weight: 700;">Important</span>
<span style="font-weight: 600;">Almost Important</span>
```

---

### Color

Color creates emphasis through contrast and meaning.

**Hierarchy via color:**

1. **High contrast:** Primary content (black on white, white on black)
2. **Medium contrast:** Secondary content (gray text)
3. **Low contrast:** Tertiary content (light gray, disabled states)

**Color for meaning:**

- **Brand color:** Primary actions, key information
- **Red:** Errors, destructive actions, urgent alerts
- **Yellow/Orange:** Warnings, caution
- **Green:** Success, confirmation, positive actions
- **Blue:** Information, links, neutral actions
- **Gray:** Inactive, disabled, de-emphasized

```css
/* ✅ CORRECT - Color hierarchy */
.primary-heading {
  color: #000000; /* Full contrast */
}

.secondary-text {
  color: #666666; /* Medium contrast */
}

.metadata {
  color: #999999; /* Low contrast */
}

/* ❌ WRONG - All same contrast */
.primary-heading { color: #333333; }
.secondary-text { color: #333333; }
.metadata { color: #333333; }
```

**Accessibility:** Maintain WCAG AA contrast ratios:

- **Normal text:** 4.5:1 minimum
- **Large text (18px+):** 3:1 minimum

---

### Position

Top-left gets most attention (Western reading pattern). Bottom-right gets least.

**F-Pattern (Content-heavy pages):**

```
┌─────────────────┐
│ ████████        │ ← Users read horizontally
│ ██              │
│ ████████        │ ← Second horizontal read
│ ██              │
│ ██              │ ← Vertical scan
└─────────────────┘
```

**Z-Pattern (Landing pages):**

```
┌──────────────►  │
│              ╲  │
│               ╲ │
│  ◄─────────────┘
```

**Application:**

- Place primary CTAs in top-right or bottom-right (end of Z/F)
- Place key information in top-left
- Use whitespace to guide eye movement

---

### Contrast

High contrast = high importance. Low contrast = low importance.

**Contrast types:**

1. **Value contrast:** Light vs. dark
2. **Color contrast:** Complementary colors
3. **Size contrast:** Large vs. small
4. **Shape contrast:** Unique shape among similar shapes

```html
<!-- ✅ CORRECT - Contrast creates hierarchy -->
<div style="background: #000; color: #fff; padding: 24px;">
  <h1>High Contrast Headline</h1>
</div>
<div style="background: #f5f5f5; color: #666; padding: 16px;">
  <p>Low contrast supporting text</p>
</div>

<!-- ❌ WRONG - No contrast hierarchy -->
<div style="background: #ddd; color: #aaa;">
  <h1>Everything looks the same</h1>
  <p>Can't tell what's important</p>
</div>
```

---

### Space

Elements with more surrounding whitespace appear more important.

**Principle:** Isolation = emphasis.

```html
<!-- ✅ CORRECT - Space creates emphasis -->
<div style="text-align: center; padding: 80px 0;">
  <h1>Featured Headline</h1>
  <p style="margin-top: 16px;">Surrounded by space, clearly important</p>
</div>

<!-- ❌ WRONG - Cramped, no emphasis -->
<div style="padding: 8px;">
  <h1>Headline</h1>
  <p>No space, gets lost</p>
</div>
```

---

## Hierarchy Patterns

### Pattern 1: Card Hierarchy

Within a card, establish clear priority.

```html
<div class="card">
  <!-- Primary: Large, bold, high contrast -->
  <h2 class="card-title" style="font-size: 24px; font-weight: 600; color: #000;">
    Main Title
  </h2>
  
  <!-- Secondary: Medium size, regular weight -->
  <p class="card-description" style="font-size: 16px; color: #333;">
    Supporting description text
  </p>
  
  <!-- Tertiary: Small, light, low contrast -->
  <span class="card-metadata" style="font-size: 14px; color: #999;">
    Posted 2 hours ago
  </span>
  
  <!-- Primary action: Most prominent button -->
  <button class="btn-primary">Read More</button>
</div>
```

**Hierarchy levels:**

1. **Title:** Most prominent
2. **Description:** Supports title
3. **Metadata:** Least important
4. **Action:** Visually distinct, actionable

---

### Pattern 2: Form Hierarchy

Guide users through form completion.

```html
<form>
  <!-- Primary: Form title -->
  <h1 style="font-size: 32px; font-weight: 700;">Create Account</h1>
  
  <!-- Secondary: Section headings -->
  <h2 style="font-size: 20px; font-weight: 600; margin-top: 24px;">
    Personal Information
  </h2>
  
  <!-- Tertiary: Field labels -->
  <label style="font-size: 14px; font-weight: 500;">Email Address</label>
  <input type="email">
  
  <!-- Quaternary: Helper text -->
  <span style="font-size: 12px; color: #666;">
    We'll never share your email
  </span>
  
  <!-- Primary action: Submit button -->
  <button type="submit" class="btn-primary">Sign Up</button>
</form>
```

---

### Pattern 3: Navigation Hierarchy

Show current location and available paths.

```html
<nav>
  <!-- Primary: Current page (bold, high contrast) -->
  <a href="/dashboard" aria-current="page" 
     style="font-weight: 600; color: #0066cc;">
    Dashboard
  </a>
  
  <!-- Secondary: Other primary nav (regular weight) -->
  <a href="/projects" style="font-weight: 400; color: #333;">
    Projects
  </a>
  
  <!-- Tertiary: Utility nav (smaller, lighter) -->
  <a href="/settings" style="font-size: 14px; color: #666;">
    Settings
  </a>
</nav>
```

---

### Pattern 4: Content Hierarchy (Article/Blog)

Guide reading flow.

```html
<article>
  <!-- Primary: Article title -->
  <h1 style="font-size: 48px; font-weight: 700; line-height: 1.2;">
    Article Headline
  </h1>
  
  <!-- Secondary: Subtitle/deck -->
  <p style="font-size: 20px; color: #555; margin-top: 8px;">
    Article subtitle providing context
  </p>
  
  <!-- Tertiary: Byline/metadata -->
  <div style="font-size: 14px; color: #999; margin-top: 16px;">
    <span>By Author Name</span>
    <span>Published Jan 1, 2026</span>
  </div>
  
  <!-- Body: Regular reading size -->
  <p style="font-size: 18px; line-height: 1.6; margin-top: 32px;">
    Body content starts here...
  </p>
  
  <!-- Secondary headings: Introduce sections -->
  <h2 style="font-size: 28px; font-weight: 600; margin-top: 40px;">
    Section Heading
  </h2>
</article>
```

---

## Common Mistakes & Fixes

### Mistake 1: Everything Is Important

**Problem:** Multiple elements compete for attention.

```html
<!-- ❌ WRONG -->
<div>
  <h1 style="font-size: 48px; font-weight: 700; color: red;">URGENT</h1>
  <h2 style="font-size: 44px; font-weight: 700; color: orange;">IMPORTANT</h2>
  <h3 style="font-size: 40px; font-weight: 700; color: yellow;">NOTICE</h3>
</div>
```

**Fix:** Establish clear priority.

```html
<!-- ✅ CORRECT -->
<div>
  <h1 style="font-size: 36px; font-weight: 600;">Primary Message</h1>
  <p style="font-size: 16px; color: #666;">Supporting information</p>
</div>
```

---

### Mistake 2: Invisible Hierarchy

**Problem:** Elements are too similar to distinguish importance.

```html
<!-- ❌ WRONG -->
<h1 style="font-size: 18px;">Page Title</h1>
<h2 style="font-size: 17px;">Section Heading</h2>
<p style="font-size: 16px;">Body text</p>
```

**Fix:** Make levels distinctly different.

```html
<!-- ✅ CORRECT -->
<h1 style="font-size: 32px; font-weight: 700;">Page Title</h1>
<h2 style="font-size: 20px; font-weight: 600;">Section Heading</h2>
<p style="font-size: 16px;">Body text</p>
```

---

### Mistake 3: Inverted Importance

**Problem:** Less important elements are more prominent.

```html
<!-- ❌ WRONG -->
<div>
  <h1 style="font-size: 14px; color: #ccc;">Main Title</h1>
  <p style="font-size: 24px; font-weight: 700; color: red;">
    Disclaimer text
  </p>
</div>
```

**Fix:** Match visual weight to actual importance.

```html
<!-- ✅ CORRECT -->
<div>
  <h1 style="font-size: 32px; font-weight: 700;">Main Title</h1>
  <p style="font-size: 12px; color: #666;">
    Disclaimer text
  </p>
</div>
```

---

### Mistake 4: Overuse of Emphasis

**Problem:** Too much bold, color, or decoration.

```html
<!-- ❌ WRONG -->
<p>
  <strong>This</strong> is <em>very</em> <strong>important</strong> 
  <span style="color: red;">information</span> that you 
  <strong>must</strong> read <em>carefully</em>!
</p>
```

**Fix:** Use emphasis sparingly.

```html
<!-- ✅ CORRECT -->
<p>
  This is important information that you must read carefully.
</p>
<!-- Or emphasize only the key point: -->
<p>
  <strong>Important:</strong> Read this information carefully.
</p>
```

---

### Mistake 5: Inconsistent Hierarchy

**Problem:** Same-level headings look different across pages.

```html
<!-- ❌ WRONG -->
<!-- Page 1 -->
<h2 style="font-size: 24px;">Section Heading</h2>

<!-- Page 2 -->
<h2 style="font-size: 18px; color: blue;">Section Heading</h2>
```

**Fix:** Use consistent styles for same levels.

```css
/* ✅ CORRECT */
h2 {
  font-size: 24px;
  font-weight: 600;
  color: #000;
}
/* Applies everywhere */
```

---

## Accessibility & Hierarchy

### Semantic HTML

Visual hierarchy should match semantic hierarchy.

```html
<!-- ✅ CORRECT -->
<h1>Page Title</h1>
  <h2>Section</h2>
    <h3>Subsection</h3>

<!-- ❌ WRONG -->
<h3>Page Title</h3>
  <h1>Section</h1>
    <h4>Subsection</h4>
```

**Principle:** Screen readers rely on heading structure. Don't skip levels.

---

### Focus Hierarchy

Keyboard navigation should follow visual hierarchy.

**Tab order should match visual importance:**

1. Primary actions first
2. Secondary actions
3. Tertiary/utility actions

```html
<!-- ✅ CORRECT tab order -->
<button tabindex="0" class="btn-primary">Save</button>
<button tabindex="0" class="btn-secondary">Cancel</button>
<a tabindex="0" class="link-tertiary">Help</a>
```

---

### Color Contrast

Hierarchy should work even without color (for colorblind users).

**Test:** Convert to grayscale. Can you still tell what's important?

```html
<!-- ✅ CORRECT - Works in grayscale -->
<h1 style="font-size: 36px; font-weight: 700;">Title</h1>
<p style="font-size: 16px;">Body</p>

<!-- ❌ WRONG - Relies only on color -->
<h1 style="color: blue;">Title</h1>
<p style="color: red;">Body</p>
```

---

## Testing Your Hierarchy

### The Squint Test

Blur your eyes (or blur screenshot). What stands out?

**Should stand out:**

- Main headings
- Primary actions
- Key information

**Should recede:**

- Body text
- Metadata
- Tertiary actions

### The 3-Second Rule

Can users identify the most important element within 3 seconds?

**Good hierarchy:** Yes, immediately obvious.  
**Bad hierarchy:** Users have to read everything to find it.

### The Scan Test

Can users scan the page and understand structure without reading?

**Good hierarchy:** Headings, spacing, and emphasis reveal structure.  
**Bad hierarchy:** Everything looks the same; must read to understand.

---

## Decision Tree

```
START: Need to establish hierarchy

1. What are the priority levels?
   - Identify 2-3 levels max per section
   - Rank by importance to user task

2. Choose hierarchy tools:
   - Primary: Large size + high contrast + bold weight
   - Secondary: Medium size + medium contrast
   - Tertiary: Small size + low contrast

3. Apply spatial hierarchy:
   - Primary: Top-left or isolated with whitespace
   - Secondary: Supporting primary
   - Tertiary: Below or aside from primary/secondary

4. Check for consistency:
   - Same-level elements use same treatment?
   - Pattern repeats across sections?

5. Validate:
   - Squint test: Does primary stand out?
   - Semantic structure: Do heading tags match visual hierarchy?
   - Accessibility: Works without color?

6. FINAL CHECK: Are there too many competing elements?
   YES → Simplify, reduce emphasis
   NO → Proceed
```

---

## Hierarchy Checklist

Before finalizing any design, verify:

- [ ] **Clear primary element:** One thing obviously most important
- [ ] **2-3 levels only:** Not more than 3 hierarchy levels per section
- [ ] **Size differences:** Each level noticeably different (20%+ size change)
- [ ] **Consistent treatment:** Same levels look the same across interface
- [ ] **Semantic alignment:** HTML heading structure matches visual hierarchy
- [ ] **Accessible contrast:** WCAG AA compliant (4.5:1 for text)
- [ ] **Works without color:** Hierarchy clear in grayscale
- [ ] **Scannable:** Can identify structure without reading every word

---

## Related Patterns

- See [Feedback & Alerting](./feedback-alerting.md) for alert hierarchy
- See [Buttons & Actions](./buttons-actions.md) for action hierarchy
- See [Layout & Spacing](./layout-spacing.md) for using space to create hierarchy

---

[← Back to Main Skill](./SKILL.md)
