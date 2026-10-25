# Historical login imagery and responsive layout

These captures record the partial layout delivered in #2895. They are historical
evidence, not acceptance of fidelity to the original MedTracker login. The
[separate correction](../login-reference/README.md) restores the original brand
and benefit icons and includes its own rendered comparison.

Reference: the four images in `rails/app/assets/images/auth`, together with
`login_layout_support.rb`, `login_brand_support.rb` and
`login_benefits_support.rb` under `rails/app/views/rodauth`.
The images are copied unchanged into `assets/static/images/auth`.

The starting Rust revision was `682320ca99e3254fd7d0d3bd917099138c206990`.
The existing [desktop login](../shared-layout/login-desktop.png) and
[mobile login](../native-auth/sign-in-mobile.png) show the original card layout.
Before-change light/dark captures and computed card styles were also recorded
during the failing acceptance test.

The new layout keeps the Rust card's background, border and shadow. It uses
side-by-side brand and form panels from 768 pixels, and stacked panels below
that width. The desktop benefits are hidden on mobile. The portrait image is
selected through 520 pixels; wider screens use the landscape image.
At that revision, benefits used the reference text without the original icons.

| Appearance | Desktop | Mobile |
| --- | --- | --- |
| Light | [Screenshot](desktop-light.png) | [Screenshot](mobile-light.png) |
| Dark | [Screenshot](desktop-dark.png) | [Screenshot](mobile-dark.png) |

The art is decorative and has empty alternative text. The existing appearance
lifecycle selects its light/dark sources after reading the saved preference,
and updates them on appearance or system changes. No image is requested before
that selection. CSS reserves its space and preserves the original proportions.
The supplied dark images retain their near-black background against the
existing charcoal card; the card colours are not changed to conceal that edge.

The focused browser test checks loaded image selection, responsive composition,
horizontal fit and automated WCAG Level A rules at 320, 390, 768 and 1440 pixels
in both appearances. It also checks saved dark mode, live system appearance
changes and password sign-in. Existing native authentication and keyboard
journeys remain part of full CI. Automated scans do not establish full WCAG
conformance or manual assistive-technology acceptance.
