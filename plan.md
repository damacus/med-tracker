# PLAN

## Theme Buttons with CSS Variables (RubyUI Theming)

Replace all hard-coded Tailwind color classes in Button/Link variants and components with semantic CSS variables, following the [RubyUI theming convention](https://rubyui.com/docs/theming).

### New CSS Variables

| Variable              | Purpose                                                      |
|-----------------------|--------------------------------------------------------------|
| `--destructive-light` | Light tint background for outline destructive buttons/badges |
| `--destructive-text`  | Dark text on light destructive backgrounds (alerts, badges)  |
| `--success-light`     | Light tint background for outline success buttons/badges     |
| `--success-text`      | Dark text on light success backgrounds                       |
| `--warning-text`      | Dark text on light warning backgrounds                       |

### Button Variants (all use CSS variables, zero hard-coded colors)

- `:primary` — `bg-primary text-primary-foreground`
- `:destructive` — `bg-destructive text-white`
- `:destructive_outline` — `text-destructive hover:bg-destructive-light`
- `:success_outline` — `text-success hover:bg-success-light`
- `:outline` — `border bg-background hover:bg-accent`
- `:secondary` — `bg-secondary text-secondary-foreground`
- `:ghost` — `hover:bg-accent`
- `:link` — `text-primary hover:underline`

### Principle

Changing the theme in `app/assets/tailwind/application.css` updates every button, alert, badge, and icon in the app — no Ruby component changes needed.
