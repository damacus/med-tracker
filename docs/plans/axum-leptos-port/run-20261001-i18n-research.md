# Rust household UI translation research

Reviewed 1 October 2026 against the current checkout and official documentation.

## Recommendation

Prefer `leptos_i18n` and `leptos_i18n_build` pinned together to `=0.6.2`, provided a bounded compile/render proof succeeds. The [official documentation](https://docs.rs/leptos_i18n/latest/leptos_i18n/) identifies 0.6.2 and compile-time translation/interpolation checks. Its [compatibility table](https://github.com/Baptistemontan/leptos_i18n/blob/master/README.md) maps 0.6.x to Leptos 0.8. Both inspected MedTracker web/UI lockfiles resolve Leptos 0.8.21. The [manifest](https://raw.githubusercontent.com/Baptistemontan/leptos_i18n/master/leptos_i18n/Cargo.toml) declares MIT and ICU-backed formatting features. Exact latest publication/commit dates were not verified: direct crates.io access failed DNS and GitHub release output was stale. Compatibility has not yet been compiled here.

Keep `config/locales/{en,cy,ga,es,pt}.yml` authoritative. Export only keys used by the new household UI into deterministic generated JSON; remove the Rails wrapper and convert parsed `%{name}` placeholders into `{{ name }}`. Reject duplicate YAML keys, unsupported values, malformed placeholders and mismatched placeholder sets. Missing English keys fail; deliberately accepted fallback requires an explicit allowlist. Do not duplicate English into other locales and describe it as translated.

## Verified application boundaries

Rails enables English-default fallback in `config/application.rb:58`. No custom plural backend, rails-i18n dependency or request locale selector was found. Existing maps predominantly contain `one` and `other`; `dashboard.dose_progress.aria_given` also contains `zero` in all five locales. No `two`, `few` or `many` maps were found.

The [Ruby I18n basic backend](https://raw.githubusercontent.com/ruby-i18n/i18n/master/lib/i18n/backend/base.rb) chooses an explicit zero branch at zero, one at one and other otherwise. [Unicode CLDR](https://www.unicode.org/cldr/charts/48/supplemental/language_plural_rules.html) gives Welsh and Irish additional categories. Direct conversion into [automatic crate plural suffixes](https://baptistemontan.github.io/leptos_i18n/declare/03_plurals.html) therefore changes semantics or encounters missing forms. Preserve existing maps through one exact-zero/one/other compatibility helper until reviewed translations support full CLDR categories.

The 30 September authentication/i18n options document says no hydration. Current `rust/web/src/dashboard.rs` includes `dashboard-hydrate.js`; `rust/ui-preview/src/lib.rs:243` hydrates a search island from a JSON dataset. Both document shells currently hard-code English. Locale must be selected once on the server and passed into render inputs and the island dataset. Hydration must use that exact locale, regardless of browser language.

Rust constructs an English greeting and raw `%A, %b %d` date in `rust/api/src/web_pages.rs:1371`. Rails dashboard also uses raw strftime. Account time zone handling already exists; language and time zone must remain independent. Date/number formatting is separate work and must not be claimed complete merely because labels are translated.

## Locale policy and alternatives

Proposed precedence: valid explicit language cookie, then weighted supported Accept-Language match (regional tags match supported base languages), then English. Reject unsupported cookie values. Set html lang and dir; all five supported languages are LTR. Initial language changes save the cookie and reload. Use explicit locale translation calls; no process-global locale mutation. The [crate resolver](https://baptistemontan.github.io/leptos_i18n/infos/01_locale_resol.html) documents SSR consistency, but the current manual to_html/hydrate_from path still needs proof.

If crate adoption cannot complete in this bounded tranche, a small typed catalogue adapter is the fallback. It should parse embedded authoritative YAML, validate its requested keys, escape user values through normal Leptos text rendering and preserve Rails plural semantics. It must expose translation gaps rather than hide them.

`rust-i18n` 4.2.3 is the second alternative: [official documentation](https://docs.rs/rust-i18n/latest/rust_i18n/) declares MIT, compile-time catalogues, Rails placeholders and explicit locale overrides. The inspected documentation does not establish CLDR/date formatting or Leptos hydration integration, so those remain application work. Global set_locale is unsuitable for concurrent request selection.

## Acceptance and effort

Test five-locale copy/interpolation; unknown/missing keys; allowed English fallback; counts 0, 1, 2, 3, 6, 7, 10, 11 and 21; escaped names; invalid cookie/header values; regional and weighted language matching; concurrent requests; SSR lang/dir and no hydration language flash. Date/number acceptance must separately cover localised weekdays/months, leap day, midnight/date rollover, London DST, decimal stock and grouping separators without modifying canonical amounts.

The estimated complete dashboard/search localisation work is 3–5 engineering days, excluding linguistic review. This run bounds implementation to the shared helper and proof needed by new household pages. Existing dashboard/search conversion and complete CLDR/date formatting remain follow-up work.
