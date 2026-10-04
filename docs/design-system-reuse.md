# Rails and Rust design-system inventory

This inventory covers the Rails source and the Rust Profile implementation in PR #2390. The change reuses existing styling; it introduces no new design values.

## Canonical sources

| Concern | Source |
| --- | --- |
| Fonts, base/light/dark tokens, palette overrides, Material 3 shapes, elevation and motion | `app/assets/tailwind/application.css` |
| Ten visible colour choices, labels and swatches | `app/views/profiles/theme_picker_card.rb` |
| Font binaries referenced by the stylesheet | `app/assets/fonts/` (matching deployed files also exist in `public/fonts/`) |
| Appearance persistence and theme classes | `app/javascript/appearance_boot.js`, `app/javascript/controllers/appearance_controller.js` |
| RubyUI base and Tailwind class merging | `app/components/ruby_ui/base.rb` |
| Local RubyUI component styling and interaction | `app/components/ruby_ui/`, `app/javascript/controllers/ruby_ui/` |
| Material 3 component variants and typography | `app/components/m3/`, `app/components/m3_helpers.rb` |
| Profile layout and individual controls | `app/views/profiles/` |

Rails has no separate Profile CSS file. Its Phlex views compose Tailwind classes with RubyUI/M3 components. The shared stylesheet also contains unrelated application rules; those are not part of the Rust export.

## Themes and exact font declarations

Every family below falls back to `sans-serif`. All declared faces are normal style, WOFF2, Latin subsets, with `font-display: swap`. Rails body uses `var(--font-family, 'Inter', sans-serif)`; the default token resolves to Plus Jakarta Sans.

| Picker option | Key | Swatch | Font | Shipped weights |
| --- | --- | --- | --- | --- |
| Command Centre | default | #1B4FB8 | Plus Jakarta Sans v12 | 400, 500, 600, 700, 800 |
| Serene Sage | serene-sage | #7DAA92 | Inter v20 | 300, 400, 500, 600, 700, 800 |
| Modern Clinical | modern-clinical | #0066FF | Plus Jakarta Sans v12 | 400, 500, 600, 700, 800 |
| Warm Earth | warm-earth | #E07A5F | Lexend v26 | 400, 500, 600, 700 |
| Deep Lavender | deep-lavender | #9B5DE5 | Inter v20 | 300, 400, 500, 600, 700, 800 |
| Forest Care | forest-care | #2D6A4F | Outfit v15 | 400, 500, 600, 700 |
| Sunset Support | sunset-support | #F28482 | Figtree v9 | 400, 500, 600, 700 |
| Tech Indigo | tech-indigo | #4361EE | Geist | No font face or binary found; generic sans-serif fallback |
| Soft Rose | soft-rose | #E5989B | Urbanist v18 | 400, 500, 600, 700 |
| Minty Fresh | minty-fresh | #06D6A0 | Public Sans v21 | 400, 500, 600, 700 |

The CSS also defines `theme-minimalist-monochrome` (#1A1A1A, Inter), but the picker does not expose it. It is retained unchanged in the export, not added to the UI. Dark Command Centre uses the source's #79A4FF primary rather than its light swatch. Theme surfaces and borders use the exact Rails OKLCH/colour-mix expressions; this table is an inventory, not a second runtime configuration.

## Material You and RubyUI customisations

The application implements Material 3 roles: primary/secondary/tertiary containers and their on-colours, error/warning/success, surface-container levels, outline, shape and elevation. The source shape scale is 4/8/12/16/28px and 9999px for full rounding. Motion durations are 150/300/500ms, with the source cubic-bezier easing. All are now exported unchanged rather than translated into new values.

RubyUI cards use `rounded-shape-xl` and elevation tokens. M3 cards add elevated/outlined/filled variants; M3 buttons add pill rounding, state layers and filled/tonal/elevated/outlined/text variants. The source state layer uses 0.08 hover and 0.12 press/focus opacity. Profile sections and sheets supply further classes in their view files. RubyUI dialogs use foreground at ten percent opacity and a 1.5px backdrop blur. RubyUI/Stimulus manages naming, focus, dismissal and scroll lock; that behaviour is separate from styling reuse.

## Tailwind configuration

Rails uses Tailwind 4.3.3 through `tailwindcss-rails` 4.6.0 / `tailwindcss-ruby` 4.3.3 in `Gemfile.lock`. Its configuration is CSS-first: `@import "tailwindcss"`, a class-based dark custom variant, `@theme inline` mappings, base/component layers and custom utilities in `application.css`. The inline mappings connect utility colours, shapes and shadows to runtime tokens. The custom container has 2rem padding and a 1400px maximum width.

Rust UI preview has its own `rust/ui-preview/style/input.css`, `tailwind.config.cjs` and Tailwind CLI 4.3.3 dependency. Its OA/Leptodon colours and Inter fallback are a separate configuration, not Rails' design system. This change does not rebuild or redesign that component library.

## Duplication found in Rust

- `rust/web/src/theme-fonts.css`: manually repeated faces, missing canonical Inter 300/800 declarations and different Inter URLs. Removed in favour of the source export.
- `rust/web/src/assets/profile.js`: repeated theme-to-font mapping. Removed; Rails' theme classes and CSS now determine the font.
- `rust/web/src/profile.css`: repeated palette hex values, light/dark surface colour mixes, default colours and swatches. Replaced with canonical tokens and picker-derived swatches.
- `rust/api/src/web_pages/assets.rs`: repeated font route tables and legacy Inter routes using copied Rust font files. The theme route now uses a source-derived embedded font table; legacy Inter URLs serve the canonical Rails bytes.
- Profile still contains hand-written layout, typography sizes, radii, shadows, breakpoints and control selectors. `auth.css`, `medication.css`, `dashboard.css` also contain independent rules. These need component-by-component mapping to the actual Phlex classes; broad replacement would be a redesign risk.

## Sharing mechanism and build contract

1. **Direct source sharing:** Rust embeds the original Rails font binaries. No runtime filesystem or network font dependency is introduced.
2. **Symlinks:** unnecessary for binaries; they do not make the mixed Tailwind source directly browser-ready and complicate restricted Docker contexts.
3. **Deterministic build export:** `rust/web/build.rs` reads the canonical stylesheet and theme picker on every relevant Cargo rebuild. It exports the complete font/base-token and runtime-palette sections unchanged, plus swatch rules derived from the picker. It embeds those outputs from Cargo's `OUT_DIR`; no generated CSS is committed or copied manually. Missing source files, fonts or changed section boundaries fail the build.
4. **Manual duplication:** retained only for the existing DOM/layout adapter pending a component migration; no new palette, font or shape constants are introduced.

The API serves the export at `/rails-design.css`; `/theme-fonts.css` remains a compatibility URL. Profile aliases existing names to Rails token names. Rust sets the same `theme-*` classes and palette-enabled attribute that Rails expects. Theme swatches are generated from the actual picker source.

The contract container copies the build script, canonical CSS, picker and fonts into its final build stage before compiling the real application. These inputs stay outside the dependency-only cache stage, so a style edit does not invalidate every compiled dependency. The disposable source-snapshot task includes those inputs. CI classification selects Rust checks when the canonical CSS, picker or fonts change. Normal clean checkout builds need Cargo only for this export; Rails, Ruby, Node and a Tailwind compilation are not needed by the export. The compiled API embeds the result and font bytes, so the running container needs no Rails checkout or shared mount.

## Separate handling

- Geist is missing from the canonical Rails assets. Do not substitute a guessed font or download one in this task.
- The hidden monochrome theme and palette-disabled Rails pages have separate rules; no new option or eligibility policy is added here.
- Existing Rails primary/on-primary colour pairs may not all meet text contrast targets. The export preserves them exactly; accessibility correction must happen in the canonical design system, not through Rust-only guessed colours.
- Tailwind utility generation, RubyUI/M3 rendering, responsive Profile layout and Stimulus behaviour are not ported by exporting tokens. They need bounded component work and matching browser tests.
- Public font copies and the dedicated Rust standalone demo server remain separate asset-delivery concerns. This change verifies the authenticated Axum Profile application and its container build.

Follow-up issues: [missing Tech Indigo font](https://github.com/damacus/med-tracker/issues/2392) and [canonical Profile dialog styling](https://github.com/damacus/med-tracker/issues/2393).

## Shared Profile overlays

`app/assets/tailwind/profile-overlays.css` is now the single browser-ready source for Profile dialogs and sheets. Rails imports it through Tailwind; Rust embeds the same file unchanged in its existing design export. Clean builds, source snapshots and the contract container include the file. There is no generated copy to synchronise.

The rules come from `RubyUI::DialogContent`, `SheetContent`, their header/title/description components and `Profiles::ProfileSettings`: centred dialog, right-hand sheet, 70% border, popover/foreground roles, elevation 5/4, 28px shape token, 10% foreground backdrop and 1.5px blur. The compiled Rails Tailwind defaults establish 24rem for `max-w-sm`, 28rem for `max-w-md`, 36rem for `max-w-xl`, .25rem spacing and the 40rem `sm` breakpoint. Profile supplies 5/6 spacing units for horizontal/body padding. The timezone dialog uses RubyUI size `sm`, which maps to `max-w-md` (28rem); appearance uses the wide sheet. Rust no longer substitutes a bottom sheet or different rounding on mobile.

The account-action forms, other Profile controls and client behaviour remain separate. Rails retains Stimulus and Rust retains its native-dialog adapter for focus, naming, dismissal and scroll lock. Sharing this stylesheet does not port the RubyUI runtime.
