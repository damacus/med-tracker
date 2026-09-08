# Interaction tranche review

## Requirements verdict: accepted for the correction tranche

The four confirmed defects meet the brief. The location member control is permanently
discoverable and its 390px keyboard cancel path restores focus. Profile labels wrap at
320px with enlarged text; button tabs now implement horizontal Arrow, Home and End
navigation. The implementation deliberately leaves server-navigation filter links alone.

The dashboard passes the existing import capability from controller to query and view.
Both downstream defaults fail closed, so an omitted capability cannot advertise the global
import. The direct route remains covered separately. Auth pages emit one inline alert when
they render one, without suppressing the login-required fallback or ordinary global notices.

The finder candidate was correctly promoted after a 320px test reproduced 105px horizontal
overflow from an unbroken source label. The narrow metadata/header wrapping change keeps the
labels and page within bounds.

The remaining mobile-rail, invitation-row and broad route/light-dark matrix work is not
accepted as complete here. It is explicitly deferred to the next verification-only tranche;
this report does not count redirects as successful coverage.

## Code-quality verdict: accepted

The shared tab implementation is proportional. It owns active selection and panel visibility,
so adding button-tab keyboard handling there avoids a profile-specific duplicate. Review caught
the existing anchor consumer used for server-side medication-review filters: the first version
would have changed its selected styling without changing the URL or results. Anchor triggers now
keep their href behaviour, while button navigation skips disabled and `aria-disabled` triggers.
The focused browser regression protects that contract.

The overflow helpers retain text-range, focus, card, menu and screenshot assertions. Their
JavaScript is held in constants with small execution helpers, addressing the method-length lint
findings without weakening the checks.

## Evidence reviewed

- Focused correction command: 86 examples, 0 failures.
- Shared-tab/filter, profile, finder and mobile-overflow command: 10 examples, 0 failures.
- Existing direct import-route authorization command: 11 examples, 0 failures.
- Coordinator RuboCop run: 1,835 files, 0 offenses.

I did not run tests or use the browser from the independent-review seat. The report accurately
distinguishes the original red component run from the source-confirmed keyboard absence and the
later browser regression. No paired new 03 before/after screenshot is claimed.
