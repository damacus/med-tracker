# Taper journey fixture

`scripts/contract_provision.rb` now seeds an independent owner-visible taper schedule for the browser journey. Its dedicated medication starts with `10.00 ml` stock and a `2.25 ml` base dose. Rails-style `schedule_config.taper_steps` supplies `1.50 ml` for the prior local day and `0.75 ml` for the effective local day through the following day. Both dates come from `Time.zone.today`, matching the Rails application timezone, and the schedule has no prior takes or timing block. A current-step dose should leave `9.25 ml` stock and create one `0.75 ml` history row.

The fixture exports the taper medication ID and name, schedule portable ID, person ID, prior/effective dates, both effective amounts and the expected starting/ending stock. It uses a separate medication from the existing desktop/mobile and API dose tests. The browser test owner owns assertions and target wiring; this lane made no product or browser-test edits.

`task rubocop` passed with 1,892 files inspected and no offenses. `git diff --check` passed. The runner will validate provisioning and the browser journey against its isolated runtime.
