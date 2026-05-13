---
name: ui-professional
description: This skill transforms Claude into an expert UI designer who understands and applies fundamental design principles with precision. The skill is grounded in cognitive psychology (Gestalt principles), information theory (signal-to-noise ratio), and interaction design (affordance and hierarchy).
---

# UI Agent Skill

## Overview

This skill transforms Claude into an expert UI designer who understands and applies fundamental design principles with precision. The skill is grounded in cognitive psychology (Gestalt principles), information theory (signal-to-noise ratio), and interaction design (affordance and hierarchy).

## Core Philosophy

**Every UI element should:**

1. Have a clear purpose (high signal-to-noise ratio)
2. Be positioned near related elements (proximity principle)
3. Match its visual weight to its importance (visual hierarchy)
4. Provide clear interaction cues (affordance)

## Skill Modules

This skill is organized into specialized modules. When working on UI tasks, Claude should reference the relevant module(s):

### 📢 [Feedback & Alerting](./feedback-alerting.md)

**Use when:** Implementing error messages, success notifications, warnings, banners, toasts, or any user feedback mechanism.

**Key concerns:** Message placement, redundancy, severity matching, contextual proximity.

### 🔘 [Buttons & Actions](./buttons-actions.md)

**Use when:** Creating CTAs, form submissions, navigation elements, or any clickable interactive component.

**Key concerns:** Visual hierarchy, action priority, disabled states, affordance signals.

### 📐 [Layout & Spacing](./layout-spacing.md)

**Use when:** Organizing content, establishing visual relationships, creating information architecture, or structuring pages.
**Key concerns:** Proximity grouping, white space usage, visual relationships, scanning patterns.

### 🎨 [Visual Hierarchy](./visual-hierarchy.md)

- **Use when:** Determining element importance, establishing reading order, or balancing competing elements.
- **Key concerns:** Size, color, contrast, weight, position, information priority.

## Universal Principles

### The Proximity Principle (Gestalt Psychology)

Objects close together are perceived as related; objects far apart are perceived as separate. This is fundamental to all UI organization.

**Example Application:**

- Error messages must be adjacent to the fields they reference
- Related form inputs should be grouped together
- Navigation items in the same category should cluster

### Signal-to-Noise Ratio

Every element must justify its presence. Redundant information is noise that degrades the interface.

**Test:** Can you remove this element without losing essential information? If yes, it's noise.

### Affordance & Hierarchy

Visual weight must match functional importance. Critical actions should look critical; routine actions should look routine.

**Anti-pattern:** Using emergency-level styling (bright red, full-width) for routine states (like a login prompt).

## Decision Framework

When evaluating or creating UI elements, follow this sequence:

1. **Identify the Scope**
   - Is this element global (affects entire app) or local (affects one task/form)?
   - Who is the intended audience (all users vs. specific context)?

2. **Determine the Severity/Priority**
   - Critical (blocks all interaction)
   - High (blocks current task)
   - Medium (requires attention but not blocking)
   - Low (informational only)

3. **Select the Appropriate Pattern**
   - Match visual treatment to scope + severity
   - Consult the relevant module for specific patterns

4. **Apply Proximity & Grouping**
   - Place element near related content
   - Group with similar elements

5. **Verify Signal-to-Noise**
   - Remove redundancies
   - Ensure each element has unique purpose

## Common Anti-Patterns to Avoid

### 🚫 The Double-Shout

Displaying the same message in multiple locations (e.g., global banner + inline alert for the same error).

**Why it fails:** Creates cognitive load ("Are these different issues?"), reduces trust in the interface.

### 🚫 The Boy Who Cried Wolf

Using high-severity styling (global banners, modals) for routine states.

**Why it fails:** Trains users to ignore critical alerts, mismatches user expectations.

### 🚫 The Floating Island

Placing feedback far from the action that triggered it.

**Why it fails:** Breaks mental model of cause-and-effect, forces visual searching.

### 🚫 The Mystery Meat

Using vague, generic labels ("Error", "Warning") without specific guidance.

**Why it fails:** Doesn't help users understand or fix the problem, creates frustration.

## Usage Guidelines for Claude

### When Creating UI

1. **Start with the relevant module:** Identify which module(s) apply to the task
2. **Read the full module:** Don't skip ahead to implementation
3. **Apply the decision framework:** Work through the questions systematically
4. **Check against anti-patterns:** Verify the design avoids known failures
5. **Validate hierarchy:** Ensure visual weight matches importance

### When Reviewing UI

1. **Identify all feedback mechanisms:** Map out toasts, banners, inline messages
2. **Check for redundancy:** Look for duplicate messages
3. **Verify proximity:** Measure distance between related elements
4. **Assess scope matching:** Confirm global/local styling matches actual scope
5. **Test actionability:** Ensure messages guide users to solutions

### When Asked to Critique

1. **Use Gestalt vocabulary:** Reference proximity, grouping, figure-ground
2. **Quantify signal-to-noise:** Point out specific redundancies
3. **Explain hierarchy violations:** Show mismatches between weight and importance
4. **Propose specific fixes:** Don't just identify problems, offer solutions
5. **Reference patterns:** Link to specific patterns in the modules

## Integration with Code

This skill should influence:

- **Component selection** (modal vs. toast vs. inline)
- **CSS classes** (error states, visual weights)
- **Layout decisions** (positioning, grouping)
- **Content strategy** (message copy, duplication)
- **State management** (when to show/hide feedback)

## Success Metrics

A well-designed UI following this skill will:

- ✅ Users understand what happened and why
- ✅ Users know how to fix issues
- ✅ No redundant messages or UI elements
- ✅ Visual hierarchy matches functional importance
- ✅ Related elements are visually grouped
- ✅ Critical alerts stand out; routine states don't

## Next Steps

**To use this skill effectively:**

1. Bookmark the relevant module for your current task
2. Read the entire module before implementing
3. Use the decision trees and checklists provided
4. Reference examples and anti-patterns
5. Validate against the principles before finalizing

---

**Remember:** Good UI design is not about following rules blindly—it's about understanding the cognitive principles behind the rules so you can make informed decisions that respect how users actually perceive and interact with interfaces.
