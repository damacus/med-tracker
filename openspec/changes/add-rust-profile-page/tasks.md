# Tasks

## 1. Behaviour and Red

- [x] 1.1 Inspect the current Rails Profile page, component actions and desktop/mobile rendering.
- [x] 1.2 Add failing tests for the canonical Profile route, four sections, time-zone dialog, Rails font and keyboard tab behaviour.
- [ ] 1.3 Add failing tests for each Security, Notifications and Advanced operation and its denial path.

## 2. Implementation

- [ ] 2.1 Finish the shared Profile shell, avatar, shortcuts, appearance, responsive layout and focus behaviour.
- [ ] 2.2 Finish Security email, password, authenticator, recovery, passkey and sign-in factor flows.
- [ ] 2.3 Finish Notifications browser push, managed people, reminder times and preference controls within Profile.
- [ ] 2.4 Finish Advanced tokens, exports, experiments, system information and account closure.
- [ ] 2.5 Keep the notification branch as the parent of this PR and update links to canonical `/profile`.

- [x] 2.6 Match the RubyUI blurred modal flow and all Rails colour themes in Light, Dark and System modes, with browser regression coverage.

## 3. Verification and delivery

- [ ] 3.1 Run focused and full applicable Rust tests, formatting, lint, contract and browser tasks.
- [ ] 3.2 Verify all sections at desktop and mobile sizes and save screenshots.
- [ ] 3.3 Review permissions, CSRF, one-use secrets, privacy, email change and closure transactions independently.
- [ ] 3.4 Validate OpenSpec and inspect the final diff for unrelated changes.
- [ ] 3.5 Commit as Dan Webb with the configured signature, push, retarget PR #2390 to the notification branch, and update its title, description and screenshots. Do not merge or deploy.
