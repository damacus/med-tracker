# Accessibility Guidelines

This document outlines the accessibility standards and implementation guidelines for MedTracker.

## WCAG 2.2 Compliance Target

MedTracker aims for **WCAG 2.2 Level AA** compliance.

## Target Size Requirements (SC 2.5.8)

### Minimum Requirements

All interactive elements (buttons, links, form controls) must meet **WCAG 2.5.8 Target Size (Minimum)**:

- **Minimum size**: 24×24 CSS pixels
- **Recommended size**: 44×44 CSS pixels (matches iOS/Android touch guidelines)

### Implementation

#### Button Component Sizes

| Size  | Height        | WCAG 2.5.8 (24px min) | Touch Recommendation (44px) |
|:------|:--------------|:----------------------|:----------------------------|
| `:sm` | 32px (`h-8`)  | ✅ Passes              | ⚠️ Below recommended        |
| `:md` | 36px (`h-9`)  | ✅ Passes              | ⚠️ Below recommended        |
| `:lg` | 40px (`h-10`) | ✅ Passes              | ⚠️ Below recommended        |
| `:xl` | 48px (`h-12`) | ✅ Passes + 2.5.5      | ✅ Meets recommendation      |

**Note:** Use `:xl` or add `min-h-[44px]` for primary mobile/touch targets.

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

1. **Inline text links** - Links within paragraphs constrained by line-height
2. **User agent controls** - Default browser form controls
3. **Essential positioning** - When size/position is fundamental to meaning

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

- [WCAG 2.5.8: Target Size (Minimum)](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html)
- [WCAG 2.5.5: Target Size (Enhanced)](https://www.w3.org/WAI/WCAG22/Understanding/target-size-enhanced.html)
- [Technique C42: Using min-height and min-width](https://www.w3.org/WAI/WCAG22/Techniques/css/C42)
