# Accessibility Guidelines

This document defines the accessibility standards and development guidance for
MedTracker.

## WCAG 2.2 Compliance Target

MedTracker aims for **WCAG 2.2 Level AA** compliance.

## Target Size Requirements (SC 2.5.8)

### Minimum Requirements

All interactive elements (buttons, links, form controls) must meet **WCAG 2.5.8 Target Size (Minimum)**:

- **Minimum size**: 24×24 CSS pixels
- **Recommended size**: 44×44 CSS pixels (matches iOS/Android touch guidelines)

### Component sizes

#### Button Component Sizes

| Size  | Height        | WCAG 2.5.8 (24px min) | Touch Recommendation (44px) |
|:------|:--------------|:----------------------|:----------------------------|
| `:sm` | 32px (`h-8`)  | ✅ Passes              | ⚠️ Below recommended        |
| `:md` | 36px (`h-9`)  | ✅ Passes              | ⚠️ Below recommended        |
| `:lg` | 40px (`h-10`) | ✅ Passes              | ⚠️ Below recommended        |
| `:xl` | 48px (`h-12`) | ✅ Passes + 2.5.5      | ✅ Meets recommendation      |

Use `:xl` or add `min-h-[44px]` for primary mobile and touch targets.

#### Touch Target Guidelines

For mobile/touch interfaces, use `min-h-[44px]` to ensure adequate touch targets:

```ruby
# Good - explicit minimum height for touch
Button(variant: :primary, class: 'min-h-[44px]') { 'Submit' }

# Good - using :xl size
Button(variant: :primary, size: :xl) { 'Submit' }
```

#### Badge-Style Buttons

When using compact badge styling, ensure minimum dimensions:

```ruby
def accessible_badge_classes
  # Touch target: min-h-[44px] min-w-[44px]
  'inline-flex items-center justify-center rounded-full ' \
    'min-h-[44px] min-w-[44px] ' \
    'px-4 py-2 text-sm font-medium'
end
```

### Spacing Requirements

When targets are undersized, ensure adequate spacing:

- Use `gap-2` (8px) or `gap-3` (12px) between adjacent targets
- A 24px diameter circle centered on each target must not intersect other targets

### Exceptions

The following are exempt from size requirements:

1. **Inline text links**: links within paragraphs constrained by line-height
2. **User agent controls**: default browser form controls
3. **Essential positioning**: use this exception when size or position is
   fundamental to meaning

## Testing

### Manual Testing

1. Draw a 24px diameter circle centered on each target
2. Verify circles don't intersect other targets
3. Test with touch devices to verify usability

### Automated Testing

Use browser dev tools to measure element dimensions:

```javascript
// In browser console
document.querySelectorAll('button, a, [role="button"]').forEach(el => {
  const rect = el.getBoundingClientRect();
  if (rect.width < 24 || rect.height < 24) {
    console.warn('Undersized target:', el, rect.width, rect.height);
  }
});
```

## References

- [WCAG 2.5.8 target size (minimum)](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html)
- [WCAG 2.5.5 target size (enhanced)](https://www.w3.org/WAI/WCAG22/Understanding/target-size-enhanced.html)
- [Technique C42 for minimum height and width](https://www.w3.org/WAI/WCAG22/Techniques/css/C42)

## Current Rust Level A audit

The Rust browser audit covers a bounded set of English journeys at desktop
(1440×1000) and mobile (390×844) sizes. It uses pinned axe-core 4.14.0 with the
`wcag2a`, `wcag21a` and `wcag22a` tags, rendered accessibility-tree snapshots and
browser keyboard interactions. Each requested page must return a successful
HTTP response at its expected URL, so an authentication redirect cannot stand
in for the intended page.

The audit identified two failures of SC 2.5.3, Label in Name: the account
control's accessible name omitted the visible person's name, and the household
chooser's name inserted the household name between the visible words “Open
household”. The corrections preserve those visible words in the accessible
name. A separate punctuation-only difference on the household switcher is not
a normative failure: W3C's [Label in Name guidance](https://www.w3.org/WAI/WCAG22/Understanding/label-in-name.html#punctuation-and-capitalization)
states that punctuation and capitalisation differences do not affect this
criterion. Removing that redundant label lets the visible text supply the name.

Page scans cover sign-in, signup and password recovery request forms, household
choice, dashboard, inventory, medication creation and detail, people, treatments,
schedule creation and profile. The signup and password recovery scans do not
complete email verification, reset confirmation or credential enrolment.
Keyboard checks exercise skip-link activation,
the invalid-login alert, mobile menu opening and Tab navigation, Escape and
focus return, the search shortcut and dismissal, and opening and dismissing the
dose dialog without recording a dose. The lower navigation check also verifies
account-dialog focus return and uses a deliberately mismatched label as a
negative control to confirm the repaired rule stays active.

Axe marks the hidden dialogs' `aria-controls` references for manual review.
The keyboard checks confirm that those targets exist as native dialogs and
exercise opening, dismissal and focus return for the account and mobile menu.

Run the focused checks with:

```fish
rtk task test:browser FILTER=accessibility_level_a
```

JSON findings, including axe items requiring manual review, and accessibility
snapshots are written to `tmp/accessibility-level-a/`. Screenshots of navigation,
the chooser, login error and dose dialog are written to
`docs/screenshots/accessibility-level-a/`. Application changes still require
`rtk task ci`; documentation changes require `rtk task docs:build` and
`rtk git diff --check`.

Reports named `negative-control-*` deliberately contain the mismatched-label
finding. They prove the detector works and are not application audit failures.

These checks do not establish WCAG conformance. VoiceOver, speech control and
the remaining journeys have not been verified by this bounded audit. Axe cannot
determine every Level A requirement, and its incomplete results need human
assessment. AA and AAA assessment is outside this change; the wider Level AA
target above remains separate.
