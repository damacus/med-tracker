# Login reference comparison

Rails is the visual baseline: [desktop](../passkey-login-idle.png),
[mobile](../login-lighthouse-mobile.png), the read-only
`rails/app/views/rodauth/login*_support.rb` views and existing auth artwork.
The mobile reference includes a passkey error; the desktop reference is idle.

This correction restores the original MT mark, benefit icons, split/stacked
composition, field icons, password recovery placement and framed alternative
sign-in action. The Rust card keeps its existing background, border and shadow.

Deliberate native-auth differences remain: Sign in leads to household selection,
Remember me is omitted because there is no corresponding Rust behaviour, and
provider sign-in appears only when configured. Appearance access is retained;
the footer does not claim a Rails application version.

Compiled candidate captures: [desktop light](desktop-light.png),
[mobile light](mobile-light.png), [desktop dark](desktop-dark.png) and
[mobile dark](mobile-dark.png). Both desktop references are idle. The candidate
mobile capture is idle; the Rails mobile reference includes a passkey error.
The capture widths match at 1440px and 375px; full-page heights differ.

The browser regression passed at 1440, 768, 390, 375 and 320px in both themes,
including image loading, overflow, card appearance, axe Level A and keyboard
password sign-in followed by the chooser's Security settings link. Manual checks
also covered a 320px invalid-password error and successful sign-in. Source and
rendered reviews approved this bounded correction.
