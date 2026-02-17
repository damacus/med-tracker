# Changelog

## [0.2.4](https://github.com/damacus/med-tracker/compare/med-tracker/v0.2.3...med-tracker/v0.2.4) (2026-02-17)


### Features

* add i18n for form components and dashboard actions ([2896c4c](https://github.com/damacus/med-tracker/commit/2896c4cbd2931cd7cf8fc14e6dc7a43a3444f1c5))
* add i18n for person medicine components ([c32a073](https://github.com/damacus/med-tracker/commit/c32a073e3a397331ee2e2a74b3807ee38a0d413f))
* add i18n translations for navigation components ([480cbcf](https://github.com/damacus/med-tracker/commit/480cbcfa7446aeb58cc4e690a5ca2affe708443f))
* **dashboard:** Finalize Family Dashboard implementation ([2334dcc](https://github.com/damacus/med-tracker/commit/2334dcc23b523d85d7a660f31b32a6288211e869))
* **dashboard:** Finalize Unified Family Dashboard with full test compatibility ([a204000](https://github.com/damacus/med-tracker/commit/a2040006fb85e1ab9ea5f3addf7e796b51df21b5))
* **dashboard:** Implement dashboard view components and translations ([0095dec](https://github.com/damacus/med-tracker/commit/0095dec46cab5d5928c6e56bf2a2844981679745))
* **dashboard:** Implement FamilyDashboard::ScheduleQuery ([35a15e3](https://github.com/damacus/med-tracker/commit/35a15e338a8ad52e788dd909658dd740d47014be))
* **dashboard:** Implementation of Family Dashboard ([665c1e9](https://github.com/damacus/med-tracker/commit/665c1e9c1cdba5d24d124471216f1304fba9a21e))
* **dashboard:** Improve ScheduleQuery with actual upcoming dose logic ([42b75b2](https://github.com/damacus/med-tracker/commit/42b75b25754be6d0b0444ef76cba2ed60167d1b3))


### Bug Fixes

* add Phlex::Rails::Helpers::T to components for i18n support ([599b1af](https://github.com/damacus/med-tracker/commit/599b1afa9b0360bc42f051affeb7dc43997db28c))
* **dashboard:** replace raw timestamp with contextual subtitle ([fa8552f](https://github.com/damacus/med-tracker/commit/fa8552fbee26b3411e158d95ab24045e3795a0f7))
* properly implement i18n support for components ([9baac95](https://github.com/damacus/med-tracker/commit/9baac95839b7bd2802b2b6f9812468ee18bdca50))
* remove markdown linting from pre-push hook ([8d255b0](https://github.com/damacus/med-tracker/commit/8d255b02ec868d5527d84f169d908ba67b9064f4))
* resolve RuboCop offenses and add comprehensive pre-push lefthook ([3a72d2d](https://github.com/damacus/med-tracker/commit/3a72d2d86b0c6780792ed982f8682710a2054fa1))
* resolve RuboCop offenses in dashboard components ([60863f0](https://github.com/damacus/med-tracker/commit/60863f0d0e0348478709b6ebe734fb18da13e5f2))
* resolve stale PID file crash on dev container startup ([fa1f80c](https://github.com/damacus/med-tracker/commit/fa1f80c182e970e7d80c841a500288d4d1d0a093))
* **tests:** Fix foreign key violations in fixtures by loading all dependencies ([183858d](https://github.com/damacus/med-tracker/commit/183858d383d13120971278306bf47c7140c53763))

## [0.2.3](https://github.com/damacus/med-tracker/compare/med-tracker/v0.2.2...med-tracker/v0.2.3) (2026-02-13)


### Features

* add authentication translations for es, cy, and pt ([#457](https://github.com/damacus/med-tracker/issues/457)) ([fbde3bd](https://github.com/damacus/med-tracker/commit/fbde3bd35e895c01cb4f2d6bd3c1df055a4946d5))
* add component tests and keyboard navigation improvements ([542fc4c](https://github.com/damacus/med-tracker/commit/542fc4cd1963c57221121cefb3536438b4d23bba))
* add optimistic UI updates for take medicine buttons [UI-010] ([1e639b7](https://github.com/damacus/med-tracker/commit/1e639b7a4c8a62d44e966dbc19de52773adbfa1b))
* add Quantity column to dashboard Medication Schedule ([#444](https://github.com/damacus/med-tracker/issues/444)) ([b7b7c2f](https://github.com/damacus/med-tracker/commit/b7b7c2f07ea555bcc2cffcffd02119a74d168527))
* add toast notifications for async take medicine actions [UI-023] ([3880412](https://github.com/damacus/med-tracker/commit/3880412b4d3651b28049193c74ef88e61b886dc0))
* disable take button when on cooldown or out of stock ([59735e3](https://github.com/damacus/med-tracker/commit/59735e3d8d4caf9a66d705c79db927be315e7acf))
* **docs:** Update documentation ([#440](https://github.com/damacus/med-tracker/issues/440)) ([9eb6c18](https://github.com/damacus/med-tracker/commit/9eb6c18cd44f4c788679bcbab96581445bef333b))
* improve form validation feedback with inline errors [UI-015] ([69d452b](https://github.com/damacus/med-tracker/commit/69d452b53f379c02534c03cff786c28e3ef45312))
* return to previous page after form submission [UI-008] ([6f33523](https://github.com/damacus/med-tracker/commit/6f33523f7ea16d4519cf0bd702e88845c7dc3732))


### Bug Fixes

* clear fixture medication_takes to prevent cooldown interference in specs ([654fa20](https://github.com/damacus/med-tracker/commit/654fa2040e0930b5c0443046647a63772669c3fe))
* revert redirect_back_or_to for create actions and admin controllers ([e93e525](https://github.com/damacus/med-tracker/commit/e93e52540fccfc49156d2e07ac0bf9e9eb9e65b5))


### Performance Improvements

* reduce test suite runtime by 19% ([#455](https://github.com/damacus/med-tracker/issues/455)) ([e4ff77e](https://github.com/damacus/med-tracker/commit/e4ff77e6a671a4b22590aa569c828825d6b8063d))

## [0.2.2](https://github.com/damacus/med-tracker/compare/med-tracker/v0.2.1...med-tracker/v0.2.2) (2026-02-09)


### Features

* add passkey/WebAuthn authentication support ([#398](https://github.com/damacus/med-tracker/issues/398)) ([4f5744a](https://github.com/damacus/med-tracker/commit/4f5744ac7d302dcab72253ca7b3973c9282d62c2))
* scrub sensitive data from OpenTelemetry traces [OTEL-014] ([#438](https://github.com/damacus/med-tracker/issues/438)) ([428524a](https://github.com/damacus/med-tracker/commit/428524a2bf37fef4156bfdc418ff8f40a02dcb5a))


### Bug Fixes

* add 44px minimum touch targets to bottom nav ([#419](https://github.com/damacus/med-tracker/issues/419)) ([7478f53](https://github.com/damacus/med-tracker/commit/7478f53b529569a1e1e3302618597dabecf9d5b1))
* add min-h-[24px] to desktop nav links for WCAG 2.5.8 compliance ([#435](https://github.com/damacus/med-tracker/issues/435)) ([770f011](https://github.com/damacus/med-tracker/commit/770f011ce434953ccbb4b27dbfa28957b19bd73a))
* add min-h-[24px] to dose counter badge for WCAG 2.5.8 compliance ([#434](https://github.com/damacus/med-tracker/issues/434)) ([207d566](https://github.com/damacus/med-tracker/commit/207d5667357cf4bb01f5231275a7540f1ea09ec4))
* add min-w-[80px] to card footer Take buttons for visual stability ([#437](https://github.com/damacus/med-tracker/issues/437)) ([9681deb](https://github.com/damacus/med-tracker/commit/9681debfe8c67974efd1c2d4674f48b9341a9bc5))
* add space between dosage amount and unit for consistency ([#430](https://github.com/damacus/med-tracker/issues/430)) ([f7825ac](https://github.com/damacus/med-tracker/commit/f7825ac0bf39f183905417be0d7e4b774b78dad7))
* apply subordinate outline style to all destructive trigger buttons ([#423](https://github.com/damacus/med-tracker/issues/423)) ([064ff87](https://github.com/damacus/med-tracker/commit/064ff87f1bbe25641af252ade897b0407c9e1f39))
* clean UI audit folder ([32dd40e](https://github.com/damacus/med-tracker/commit/32dd40e3669d989b569d3e19e871e8d85f5058b8))
* **deps:** update playwright monorepo to v1.58.1 ([#401](https://github.com/damacus/med-tracker/issues/401)) ([d56d0a4](https://github.com/damacus/med-tracker/commit/d56d0a4a8c6bf65845daf2a8e776e035d1365067))
* **deps:** update playwright monorepo to v1.58.2 ([#424](https://github.com/damacus/med-tracker/issues/424)) ([02ae409](https://github.com/damacus/med-tracker/commit/02ae40914d0f322f4d1975510fe83cd514d217cd))
* display correct dosage unit instead of hardcoded ml ([#421](https://github.com/damacus/med-tracker/issues/421)) ([61b778e](https://github.com/damacus/med-tracker/commit/61b778ee01d7ea21264e6e24addf4eab94c1c699))
* redesign mobile hamburger menu as slide-out drawer ([#439](https://github.com/damacus/med-tracker/issues/439)) ([2f7370c](https://github.com/damacus/med-tracker/commit/2f7370c4c60db3b6ff9da0c5efa5555db8095deb))
* reduce Delete button prominence on prescription cards ([#422](https://github.com/damacus/med-tracker/issues/422)) ([231042d](https://github.com/damacus/med-tracker/commit/231042dc00cff3441a2c0484f2a395a3ccb7242d))
* reduce Delete button visual weight on dashboard ([#417](https://github.com/damacus/med-tracker/issues/417)) ([cbb9835](https://github.com/damacus/med-tracker/commit/cbb98357424bf838d1f53b3b7e2a4c0d49829aae))
* remove duplicate flash on login page (keep inline, suppress global) ([#413](https://github.com/damacus/med-tracker/issues/413)) ([4269349](https://github.com/damacus/med-tracker/commit/426934961a6aeae16393ee5803ef2d46e02dae3e))
* remove redundant Current Supply card from medicine show ([#418](https://github.com/damacus/med-tracker/issues/418)) ([1068c99](https://github.com/damacus/med-tracker/commit/1068c99b4bbb4ac93b11963c01bde9c4fe4786c3))
* remove redundant Success/Error titles from flash messages ([#411](https://github.com/damacus/med-tracker/issues/411)) ([d6f6960](https://github.com/damacus/med-tracker/commit/d6f69605f12ce997af41615291afce9f5ecc918c))
* replace raw CSS button classes with RubyUI Link variants in dashboard ([#433](https://github.com/damacus/med-tracker/issues/433)) ([f57b372](https://github.com/damacus/med-tracker/commit/f57b3725a1d2e037046694ebc86d78498832895d))
* standardize medicine icon color to violet across medication cards ([#436](https://github.com/damacus/med-tracker/issues/436)) ([c6b5f21](https://github.com/damacus/med-tracker/commit/c6b5f21036334d653b4f92e8eb31fed1f35e4dc6))
* standardize Take button size to :md across medication cards ([#432](https://github.com/damacus/med-tracker/issues/432)) ([678b307](https://github.com/damacus/med-tracker/commit/678b3070d39804627f026a9ad55115b1065eb4d8))
* UI quick fixes batch - accessibility, consistency, and cleanup ([#425](https://github.com/damacus/med-tracker/issues/425)) ([2df3672](https://github.com/damacus/med-tracker/commit/2df367234efbfec0898e2d62c6d28900332fab88))
* use amber/warning styling for countdown notices distinct from notes ([#431](https://github.com/damacus/med-tracker/issues/431)) ([00b2503](https://github.com/damacus/med-tracker/commit/00b250336ce88d86d2cf7ef2c5ec4a9b294afef2))
* use notice flash for login redirect instead of error ([#416](https://github.com/damacus/med-tracker/issues/416)) ([12c69e3](https://github.com/damacus/med-tracker/commit/12c69e3857b1bd8854d88a6d58aff103fd312b09))
* use RubyUI::Badge for Needs Carer badge instead of inline styles ([#420](https://github.com/damacus/med-tracker/issues/420)) ([b7578bc](https://github.com/damacus/med-tracker/commit/b7578bcf2d7742889ca7aa2957d34829c178ec41))
* use warning variant for 2FA setup notice instead of success ([#415](https://github.com/damacus/med-tracker/issues/415)) ([81ab60f](https://github.com/damacus/med-tracker/commit/81ab60fd4e1c06aea29872b23149d5d926a8a1ac))

## [0.2.1](https://github.com/damacus/med-tracker/compare/med-tracker/v0.2.0...med-tracker/v0.2.1) (2026-01-26)


### Bug Fixes

* **deps:** update playwright monorepo to v1.58.0 ([#393](https://github.com/damacus/med-tracker/issues/393)) ([9d8cce1](https://github.com/damacus/med-tracker/commit/9d8cce1d65b88261c9daff218a0438340233a242))
* improve mobile button styling and layout ([#392](https://github.com/damacus/med-tracker/issues/392)) ([4874155](https://github.com/damacus/med-tracker/commit/4874155a1c6c149633896230d0dbc8b25fd54ab4))

## [0.2.0](https://github.com/damacus/med-tracker/compare/med-tracker-v0.1.0...med-tracker/v0.2.0) (2026-01-22)


### ⚠ BREAKING CHANGES

* **auth:** Person type simplified from 7 types to 3 (adult, minor, dependent_adult)

### Features

* Add OpenTelemetry OTLP exporter and HTTP metrics instrumentation ([#194](https://github.com/damacus/med-tracker/issues/194)) ([74e4ad9](https://github.com/damacus/med-tracker/commit/74e4ad9f56bff9274143294139c33b62b90fb2cb))
* add timing restriction enforcement to prescription cards ([a207295](https://github.com/damacus/med-tracker/commit/a207295eaaba7f659caaf225c0ce4d8fd4bee8a8))
* **admin:** add user management controller ([f816104](https://github.com/damacus/med-tracker/commit/f816104a04987b788edc9b92cd854eaf2e448a01))
* Audit trail ([#137](https://github.com/damacus/med-tracker/issues/137)) ([edf5882](https://github.com/damacus/med-tracker/commit/edf58826a2585d3c924190ae1ec9d606cf8a1ac3))
* **audit:** implement AUDIT-013 and AUDIT-014 features ([#175](https://github.com/damacus/med-tracker/issues/175)) ([39d9b3b](https://github.com/damacus/med-tracker/commit/39d9b3b730941fe6c57ef358a95e828c9201c1fc))
* **auth:** add passkeys-rails for passwordless authentication ([f816104](https://github.com/damacus/med-tracker/commit/f816104a04987b788edc9b92cd854eaf2e448a01))
* **auth:** add Pundit authorisation and passkey authentication ([f816104](https://github.com/damacus/med-tracker/commit/f816104a04987b788edc9b92cd854eaf2e448a01))
* **auth:** add Pundit authorisation framework ([f816104](https://github.com/damacus/med-tracker/commit/f816104a04987b788edc9b92cd854eaf2e448a01))
* **auth:** implement Rodauth authentication with Google OAuth support ([#172](https://github.com/damacus/med-tracker/issues/172)) ([fce0332](https://github.com/damacus/med-tracker/commit/fce03325008e1357038c103c889d30ffab987c6e))
* **auth:** implement Rodauth signup with capacity-based carer validation ([#164](https://github.com/damacus/med-tracker/issues/164)) ([38009c5](https://github.com/damacus/med-tracker/commit/38009c5c5e403e4ae905f977cc3e9525f9f63cdf))
* **auth:** implement role-based authorisation policies ([f816104](https://github.com/damacus/med-tracker/commit/f816104a04987b788edc9b92cd854eaf2e448a01))
* **carer:** implement relationship reactivation (CARER-015) ([#171](https://github.com/damacus/med-tracker/issues/171)) ([809890d](https://github.com/damacus/med-tracker/commit/809890d1ccdbc3ba8ef779e3b4ec7bcb2771ad35))
* complete admin interface with dashboard, user crud, and search ([#104](https://github.com/damacus/med-tracker/issues/104)) ([7a21a51](https://github.com/damacus/med-tracker/commit/7a21a51f72d0909a1caf65335c94eedb5c22ea66))
* complete authorization implementation for medication tracking ([#101](https://github.com/damacus/med-tracker/issues/101)) ([28d6711](https://github.com/damacus/med-tracker/commit/28d6711f813aed393a38cda352bf2c4d744eb186))
* consolidate action buttons in card footers ([a207295](https://github.com/damacus/med-tracker/commit/a207295eaaba7f659caaf225c0ce4d8fd4bee8a8))
* **domain:** simplify person_type model ([f816104](https://github.com/damacus/med-tracker/commit/f816104a04987b788edc9b92cd854eaf2e448a01))
* Enable self-management of OTC medicines and admin user creation with Rodauth accounts ([#176](https://github.com/damacus/med-tracker/issues/176)) ([b9519d8](https://github.com/damacus/med-tracker/commit/b9519d86ff72fcd31ce0921a93e52303133cbb7e))
* extract timing restrictions logic into reusable concern ([#129](https://github.com/damacus/med-tracker/issues/129)) ([a207295](https://github.com/damacus/med-tracker/commit/a207295eaaba7f659caaf225c0ce4d8fd4bee8a8))
* implement comprehensive authorization system with Pundit and passkey authentication ([#76](https://github.com/damacus/med-tracker/issues/76)) ([f816104](https://github.com/damacus/med-tracker/commit/f816104a04987b788edc9b92cd854eaf2e448a01))
* implement ECS logging format for production observability ([fd77f01](https://github.com/damacus/med-tracker/commit/fd77f014eff511e801754721ad0ae8d6a58d3ee3))
* implement medicine stock tracking and visibility ([6db5f17](https://github.com/damacus/med-tracker/commit/6db5f17b4daab3ce2dded1b7a07566dba48397dd))
* Implement user invitation system (INVITE-001, INVITE-002) ([#226](https://github.com/damacus/med-tracker/issues/226)) ([4d5fe62](https://github.com/damacus/med-tracker/commit/4d5fe6208b5eac7e6a9810edfcf5e244907ea239))
* improve people page navigation and carer assignment ([#206](https://github.com/damacus/med-tracker/issues/206)) ([a79026e](https://github.com/damacus/med-tracker/commit/a79026e0252bac002ab09ca823038a4ad8759e0d))
* Improve prescription form selects and calendar picker ([#184](https://github.com/damacus/med-tracker/issues/184)) ([8b4a448](https://github.com/damacus/med-tracker/commit/8b4a4482acc9f81fb09d9c6288aa6b44f4e1557d))
* **medications:** add non-prescription medication support ([f816104](https://github.com/damacus/med-tracker/commit/f816104a04987b788edc9b92cd854eaf2e448a01))
* move countdown display to prominent notice section ([a207295](https://github.com/damacus/med-tracker/commit/a207295eaaba7f659caaf225c0ce4d8fd4bee8a8))
* OpenTelemetry observability ([#170](https://github.com/damacus/med-tracker/issues/170)) ([563b54b](https://github.com/damacus/med-tracker/commit/563b54bb76be3405a2199314034dcb4e56678fcc))
* reorganise person page layout ([a207295](https://github.com/damacus/med-tracker/commit/a207295eaaba7f659caaf225c0ce4d8fd4bee8a8))
* **security:** comprehensive security hardening pass ([#173](https://github.com/damacus/med-tracker/issues/173)) ([097776c](https://github.com/damacus/med-tracker/commit/097776cb0f986b7b1dfc53bc8436430d0dac5e79))
* Simplify and refactor database schema ([#9](https://github.com/damacus/med-tracker/issues/9)) ([3aab84f](https://github.com/damacus/med-tracker/commit/3aab84f93a81817e0530d6e2b6c01f246adaf6ce))
* **ui:** add authorisation-aware UI components ([f816104](https://github.com/damacus/med-tracker/commit/f816104a04987b788edc9b92cd854eaf2e448a01))
* **ui:** Standardize forms with RubyUI components and optimize mobile dashboard ([#186](https://github.com/damacus/med-tracker/issues/186)) ([e07d66c](https://github.com/damacus/med-tracker/commit/e07d66cc8309dc50215fad40f3ffd975cba237be))
* update coding-agent workflow to use Beads (bd) ([f795593](https://github.com/damacus/med-tracker/commit/f795593d68e8aadc684564a762c4f79101e21804))
* User Management ([#148](https://github.com/damacus/med-tracker/issues/148)) ([27aac4c](https://github.com/damacus/med-tracker/commit/27aac4c3381707d9af6c9c27eafcdb6e3b3aeb6c))


### Bug Fixes

* Add tests ([#20](https://github.com/damacus/med-tracker/issues/20)) ([b72cb79](https://github.com/damacus/med-tracker/commit/b72cb7959700326eba247618ff81ab0fc75dcf4b))
* **auth:** fix critical authorisation policy bugs ([f816104](https://github.com/damacus/med-tracker/commit/f816104a04987b788edc9b92cd854eaf2e448a01))
* **auth:** resolve authorization test failures and fixture issues ([#102](https://github.com/damacus/med-tracker/issues/102)) ([2068e37](https://github.com/damacus/med-tracker/commit/2068e37d84dd6df01ec7b6539982d7afdc3767ac))
* correct heading hierarchy for screen reader accessibility (A11Y-012) ([#199](https://github.com/damacus/med-tracker/issues/199)) ([9c47315](https://github.com/damacus/med-tracker/commit/9c47315feb324f0243474222a2e6a8eafa336591))
* correct stock tracking tests to use existing medicine ([bafa5f0](https://github.com/damacus/med-tracker/commit/bafa5f0754d64cbe2beb74c892da65d301efa3ab))
* **deps:** update playwright monorepo to v1.56.1 ([#77](https://github.com/damacus/med-tracker/issues/77)) ([4583160](https://github.com/damacus/med-tracker/commit/4583160897f439cfbc290d8a42b7ba4235e98dd8))
* **deps:** update playwright monorepo to v1.57.0 ([#153](https://github.com/damacus/med-tracker/issues/153)) ([d8dd871](https://github.com/damacus/med-tracker/commit/d8dd8719d16ba59ccd11a05b40e8ee4bdd7b1005))
* disable Rails/SkipsModelValidations for atomic stock updates ([bc20cb0](https://github.com/damacus/med-tracker/commit/bc20cb012ad57ffd4189ebf424e43219a0dcdf01))
* guard EcsLogging::Logger for asset precompilation ([#391](https://github.com/damacus/med-tracker/issues/391)) ([5ff5bc7](https://github.com/damacus/med-tracker/commit/5ff5bc710f54735909615e2f1fd790f9f8a2dab8))
* Person medicine policy ([#99](https://github.com/damacus/med-tracker/issues/99)) ([380d99f](https://github.com/damacus/med-tracker/commit/380d99fc2b64ad2ec2bea1534df7767046b01535))
* remove decimal places from all dosage displays ([a207295](https://github.com/damacus/med-tracker/commit/a207295eaaba7f659caaf225c0ce4d8fd4bee8a8))
* remove stale PID file before starting Rails server in dev ([#388](https://github.com/damacus/med-tracker/issues/388)) ([e32e17b](https://github.com/damacus/med-tracker/commit/e32e17bb63650c0223380db6c417a3590c6938b9))
* resolve merge conflicts in audit logs components and specs ([#182](https://github.com/damacus/med-tracker/issues/182)) ([1c20380](https://github.com/damacus/med-tracker/commit/1c20380f37b93134757ce7cb0aa7ca25fe0e9195))
* **security:** change default user role from administrator to parent ([f816104](https://github.com/damacus/med-tracker/commit/f816104a04987b788edc9b92cd854eaf2e448a01))
* update AlertDialog cancel buttons to use Button component ([a207295](https://github.com/damacus/med-tracker/commit/a207295eaaba7f659caaf225c0ce4d8fd4bee8a8))
* use blank? instead of unless present? in medication_take ([a725912](https://github.com/damacus/med-tracker/commit/a7259121f3c9c5f6459e057e881ae53b491e3882))
* use named subject and fix line length in medicine spec ([74ba483](https://github.com/damacus/med-tracker/commit/74ba4839b75539df9c0fc150df5e542b13190570))

## Changelog
