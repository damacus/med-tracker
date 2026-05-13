# Layout & Spacing

[← Back to Main Skill](./SKILL.md)

## Purpose

This module governs spatial relationships, grouping, whitespace, and information architecture. The goal is to create clear visual relationships that help users understand content structure and navigate efficiently.

---

## Core Principles

### 1. The Proximity Principle (Gestalt)

Elements that are close together are perceived as related; elements far apart are perceived as separate.

**Reasoning:** Our brains automatically group nearby objects. This is not a design choice—it's cognitive psychology. Fight it at your users' peril.

**Application:** Related elements should have less space between them than unrelated elements.

```
✅ CORRECT spacing:
┌─────────────────┐
│ Name            │
│ Email           │ ← 8px gap (related)
├─────────────────┤ ← 24px gap (section break)
│ Preferences     │
│ Notifications   │ ← 8px gap (related)
└─────────────────┘

❌ WRONG spacing:
┌─────────────────┐
│ Name            │
├─────────────────┤ ← 24px gap (suggests separation)
│ Email           │ ← but these are related!
├─────────────────┤
│ Preferences     │
├─────────────────┤ ← same gap (no hierarchy)
│ Notifications   │
└─────────────────┘
```

### 2. Whitespace as Communication

Empty space is not wasted space—it creates breathing room, establishes hierarchy, and guides the eye.

**Reasoning:** Dense layouts increase cognitive load. Strategic whitespace improves scanning and comprehension.

### 3. Consistent Spacing Scale

Use a limited set of spacing values that follow a predictable pattern.

**Reasoning:** Consistent spacing creates rhythm and predictability. Random spacing creates visual chaos.

**Common scale (8px base):**

- xs: 4px (tight grouping)
- sm: 8px (related elements)
- md: 16px (section separation)
- lg: 24px (major section separation)
- xl: 32px (content area separation)
- xxl: 48px+ (page-level separation)

### 4. Visual Weight Distribution

Balance the "weight" of elements across the layout. Dense areas need offsetting whitespace.

**Reasoning:** Unbalanced layouts feel uncomfortable and direct attention poorly.

---

## Spacing Patterns

### Pattern 1: Form Field Groups

Related inputs should cluster; sections should separate.

```html
<form>
  <!-- Personal info section -->
  <div class="form-section">
    <h2>Personal Information</h2>
    
    <div class="form-field">
      <label for="first-name">First Name</label>
      <input id="first-name" type="text">
    </div>
    <!-- 8-12px gap -->
    
    <div class="form-field">
      <label for="last-name">Last Name</label>
      <input id="last-name" type="text">
    </div>
    <!-- 8-12px gap -->
    
    <div class="form-field">
      <label for="email">Email</label>
      <input id="email" type="email">
    </div>
  </div>
  <!-- 24-32px gap (section break) -->
  
  <!-- Account preferences section -->
  <div class="form-section">
    <h2>Account Preferences</h2>
    <!-- More fields -->
  </div>
</form>
```

**Spacing ratios:**

- Label to input: 4px
- Input to helper text: 4px
- Field to field (same section): 12px
- Section to section: 24-32px

---

### Pattern 2: Card Layouts

Cards group related content; gaps between cards show separation.

```html
<div class="card-grid">
  <div class="card">
    <!-- Card content with internal padding -->
  </div>
  <!-- 16-24px gap -->
  
  <div class="card">
    <!-- Card content -->
  </div>
</div>
```

**Guidelines:**

- Internal padding: 16-24px (creates breathing room)
- Gap between cards: 16-24px (shows separation)
- Grid cards: Use equal gaps for visual rhythm

---

### Pattern 3: Content Hierarchy

Use spacing to establish reading order and section importance.

```html
<article>
  <h1>Main Title</h1>
  <!-- 8px gap (h1 to subtitle) -->
  <p class="subtitle">Article subtitle</p>
  <!-- 16px gap (subtitle to metadata) -->
  <div class="metadata">
    <span>By Author</span>
    <span>Date</span>
  </div>
  <!-- 32px gap (header to body) -->
  
  <p>First paragraph...</p>
  <!-- 16px gap (paragraph to paragraph) -->
  <p>Second paragraph...</p>
  
  <h2>Section Heading</h2>
  <!-- 16px gap (heading to content) -->
  <p>Section content...</p>
</article>
```

**Hierarchy rules:**

- Title to subtitle: Tight (8px) - they're a unit
- Metadata to body: Generous (32px) - marks content start
- Paragraph to paragraph: Medium (16px) - related but distinct
- Section heading to content: Medium (16px) - introduces new idea

---

### Pattern 4: Navigation & Menus

Menu items cluster; menu sections separate.

```html
<nav>
  <!-- Primary navigation -->
  <ul class="nav-list">
    <li><a href="/">Home</a></li>
    <li><a href="/about">About</a></li>
    <li><a href="/services">Services</a></li>
  </ul>
  <!-- 24px gap (section separator) -->
  
  <!-- Account section -->
  <ul class="nav-list">
    <li><a href="/profile">Profile</a></li>
    <li><a href="/settings">Settings</a></li>
    <li><a href="/logout">Logout</a></li>
  </ul>
</nav>
```

**Navigation spacing:**

- Item to item (same section): 4-8px
- Section to section: 16-24px
- Use dividers or spacing, not both

---

## Layout Systems

### Grid-Based Layouts

Use CSS Grid for 2D layouts (rows and columns).

```css
.dashboard {
  display: grid;
  grid-template-columns: 250px 1fr;
  gap: 24px; /* Consistent gap */
}

.card-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 20px;
}
```

**When to use:**

- Dashboard layouts
- Card grids
- Complex multi-column pages
- Asymmetric layouts

---

### Flexbox-Based Layouts

Use Flexbox for 1D layouts (single row or column).

```css
.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
}

.button-group {
  display: flex;
  gap: 12px;
}
```

**When to use:**

- Navigation bars
- Button groups
- Single-row/column arrangements
- Centering content

---

### Container Patterns

#### Fixed-Width Centered

For text-heavy content (optimal reading line length).

```css
.content {
  max-width: 680px; /* 60-80 characters per line */
  margin: 0 auto;
  padding: 0 24px; /* Breathing room on mobile */
}
```

#### Full-Width with Constrained Content

For visual impact with readable content.

```css
.section {
  width: 100%;
  background: lightblue; /* Full-width bg */
}

.section-content {
  max-width: 1200px;
  margin: 0 auto;
  padding: 48px 24px; /* Constrained content */
}
```

#### Sidebar Layout

For navigation + content.

```css
.layout {
  display: grid;
  grid-template-columns: 250px 1fr;
  gap: 24px;
  min-height: 100vh;
}
```

---

## Responsive Spacing

Spacing should adapt to screen size:

```css
.section {
  padding: 16px; /* Mobile */
}

@media (min-width: 768px) {
  .section {
    padding: 32px; /* Tablet */
  }
}

@media (min-width: 1024px) {
  .section {
    padding: 48px; /* Desktop */
  }
}
```

**Principle:** Tighten spacing on small screens; increase on large screens.

---

## Common Mistakes & Fixes

### Mistake 1: Equal Spacing Everywhere

**Problem:** All gaps are the same, creating no hierarchy.

```css
/* ❌ WRONG */
.form-field { margin-bottom: 20px; }
.form-section { margin-bottom: 20px; }
/* Everything looks equally related */
```

**Fix:** Use proximity to show relationships.

```css
/* ✅ CORRECT */
.form-field { margin-bottom: 12px; } /* Related fields */
.form-section { margin-bottom: 32px; } /* Section break */
```

---

### Mistake 2: No Breathing Room

**Problem:** Elements are too close, creating visual clutter.

```html
<!-- ❌ WRONG -->
<div class="card" style="padding: 4px;">
  <h3>Title</h3>
  <p>Content immediately below title</p>
  <button>Action</button>
</div>
```

**Fix:** Add strategic whitespace.

```html
<!-- ✅ CORRECT -->
<div class="card" style="padding: 24px;">
  <h3>Title</h3>
  <!-- 8px gap -->
  <p>Content with breathing room</p>
  <!-- 16px gap -->
  <button>Action</button>
</div>
```

---

### Mistake 3: Breaking Visual Groups

**Problem:** Large gaps within related content.

```html
<!-- ❌ WRONG -->
<div class="form-group">
  <label>Email</label>
  <!-- 20px gap - too much! -->
  <input type="email">
</div>
```

**Fix:** Keep related elements close.

```html
<!-- ✅ CORRECT -->
<div class="form-group">
  <label>Email</label>
  <!-- 4px gap - clearly connected -->
  <input type="email">
</div>
```

---

### Mistake 4: Inconsistent Scale

**Problem:** Random spacing values (13px, 17px, 22px...).

```css
/* ❌ WRONG */
.header { padding: 13px; }
.section { padding: 17px; }
.footer { padding: 22px; }
```

**Fix:** Use a consistent spacing scale.

```css
/* ✅ CORRECT */
.header { padding: 12px; }   /* sm */
.section { padding: 16px; }  /* md */
.footer { padding: 24px; }   /* lg */
```

---

### Mistake 5: Ignoring Responsive Needs

**Problem:** Same spacing on mobile as desktop.

```css
/* ❌ WRONG */
.hero {
  padding: 80px 24px; /* Giant on mobile */
}
```

**Fix:** Scale spacing down on small screens.

```css
/* ✅ CORRECT */
.hero {
  padding: 32px 16px; /* Mobile */
}

@media (min-width: 768px) {
  .hero {
    padding: 80px 24px; /* Desktop */
  }
}
```

---

## Alignment & Positioning

### Alignment Principles

**Left-align text:** For readability (especially multi-line).

```css
.text-content {
  text-align: left; /* Easier to scan */
}
```

**Center-align sparingly:** For headings, hero sections, CTAs.

```css
.hero-heading {
  text-align: center; /* Draws focus */
}
```

**Avoid justified text:** Creates uneven word spacing (especially in narrow columns).

---

### Vertical Alignment

Use consistent alignment within horizontal groups.

```html
<!-- ✅ CORRECT -->
<div class="header" style="display: flex; align-items: center;">
  <img src="logo.png" alt="Logo">
  <h1>Site Title</h1>
  <nav>...</nav>
</div>
```

**Principle:** Align baselines for text, centers for mixed content.

---

## Visual Rhythm

### Repetition & Pattern

Consistent spacing creates visual rhythm that's easy to scan.

```html
<!-- ✅ Good rhythm -->
<div class="article-list">
  <article>...</article> <!-- 24px gap -->
  <article>...</article> <!-- 24px gap -->
  <article>...</article> <!-- 24px gap -->
</div>
```

**Benefits:**

- Predictable scanning pattern
- Feels organized and professional
- Reduces cognitive load

---

### Breaking the Pattern

Strategic breaks in rhythm signal importance.

```html
<div class="content">
  <section>...</section> <!-- 24px gap -->
  <section>...</section> <!-- 24px gap -->
  
  <!-- 48px gap - signals major shift -->
  
  <section class="highlight">...</section>
</div>
```

**Use for:**

- Featured content
- Major section transitions
- Calls to action

---

## Accessibility Considerations

### Focus Indicators & Spacing

Ensure focus rings don't overlap with adjacent elements.

```css
button:focus-visible {
  outline: 2px solid blue;
  outline-offset: 2px; /* Space prevents overlap */
}
```

### Touch Targets & Spacing

Ensure adequate space between interactive elements (especially mobile).

**Minimum:** 44×44px touch targets with 8px spacing between.

```html
<nav class="mobile-nav">
  <a href="/" style="padding: 12px;">Home</a> <!-- 8px gap -->
  <a href="/about" style="padding: 12px;">About</a> <!-- 8px gap -->
  <a href="/contact" style="padding: 12px;">Contact</a>
</nav>
```

---

## Testing Your Layout

Ask these questions:

1. **Proximity:** Are related elements closer than unrelated ones?
2. **Hierarchy:** Does spacing reflect content importance?
3. **Consistency:** Am I using a limited set of spacing values?
4. **Breathing room:** Does the layout feel cramped or comfortable?
5. **Responsive:** Does spacing adapt appropriately to screen size?
6. **Alignment:** Are elements aligned in a predictable way?
7. **Accessibility:** Are touch targets large enough and spaced apart?

---

## Decision Tree

```
START: Need to determine spacing

1. Are these elements related?
   YES → Small gap (4-12px)
   NO → Continue to 2

2. Are they in the same section?
   YES → Medium gap (16-24px)
   NO → Large gap (32-48px)

3. Is this a major content break?
   YES → Extra large gap (48px+)
   NO → Use standard section gap

4. Am I on mobile?
   YES → Reduce spacing by 25-50%
   NO → Use full spacing scale

5. FINAL CHECK: Does the spacing create clear visual groups?
   YES → Proceed
   NO → Adjust to strengthen proximity relationships
```

---

## Spacing Scale Reference

Use this scale consistently throughout your designs:

| Name | Value | Use For |
|------|-------|---------|
| **xs** | 4px | Label to input, icon to text |
| **sm** | 8px | Related list items, tight groupings |
| **md** | 16px | Paragraph spacing, card padding |
| **lg** | 24px | Section separators, card gaps |
| **xl** | 32px | Major section breaks |
| **2xl** | 48px | Page-level separation |
| **3xl** | 64px+ | Hero sections, dramatic breaks |

---

## Related Patterns

- See [Feedback & Alerting](./feedback-alerting.md) for alert positioning in layouts
- See [Buttons & Actions](./buttons-actions.md) for button group spacing
- See [Visual Hierarchy](./visual-hierarchy.md) for using space to create emphasis

---

[← Back to Main Skill](./SKILL.md)
