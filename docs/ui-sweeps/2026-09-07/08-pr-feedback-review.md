# PR 2126 feedback review

Independent Sol review found no code, test, or screenshot findings.

- The show-view wrapper correctly owns direct-child grid placement. The
  inventory modal already composes RubyUI Dialog primitives; keep it unchanged.
- Card hover shadow remains. Restoring hover scaling would recreate a
  containing block around the fixed dropdown and risk clipping; keep it removed.
- Removing the shared dropdown root's elevation keeps closed triggers under
  navigation. The menu content retains its overlay layer for both fixed card
  menus and the absolute profile menu.
- The new browser regression tests the topmost element at an actual overlap
  between each card Actions trigger and a navigation link. It failed for the
  original defect and passed after the shared fix.
- Refreshed 390px light/dark screenshots show navigation above card content;
  1280px light/dark screenshots preserve the desktop action layout.

Focused browser and component tests pass; Ruby lint inspected 1,824 files with
no offenses. Documentation build passes. The follow-up review also accepted the
single visible-profile-panel readiness wait without relaxing geometry checks.
Final full suite passed: 5,515 examples, zero failures, one existing
OIDC-dependent pending example. Accepted for commit and push.
