# Authentication compatibility execution — 1 October 2026

## Completed safe slice

This tranche supplies a pure, explicit-input OTP compatibility helper and deterministic tests. It does not load Rails secrets, query factor rows, mutate authentication state, grant sessions or enable existing-factor login. The full authentication parity change remains open.

The proposal and research retain existing account IDs, schema and signed database-backed sessions. Accepted policy from `unify-mobile-login-with-rodauth` remains 30-day interactive inactivity with no default absolute maximum, optional MFA, no action-specific fresh-MFA gates, app tokens capped at 12 calendar months and account-level native authentication with per-request household authorisation.

## Rails oracle evidence

The oracle ran through a one-off inline `rtk proxy task --taskfile - oracle` task using the locally installed Ruby 4.0.7 executable and the repository's locked Rodauth 2.48.0/ROTP 6.3.0 gems. The repository specifies Ruby 4.0.6; this host patch differs, so the result proves the locked gem algorithms against the recorded synthetic data rather than the full Rails runtime. All seeds and HMAC inputs below are synthetic public test data.

Current HMAC input: `synthetic-rails-secret-key-base-for-compatibility`. Old HMAC input: `synthetic-old-rails-secret-key-base`. Fixed Unix time: `1700000000`.

| Stored seed | Generation | Derived authenticator secret | Code at fixed time |
| --- | --- | --- | --- |
| `abcdefghijklmnop` | Current | `4tlxacqrzvpp3asc` | `649638` |
| `abcdefghijklmnop` | Old | `wdhelz44632d4ntv` | `621095` |
| `abcdefghijklmnopqrstuvwxyz234567` | Current | `ugpjb54nzdldum6kwhirumb2j65ga6fl` | `055041` |
| `abcdefghijklmnopqrstuvwxyz234567` | Old | `r6nuz5lg2oracz6lm6c7do2pxgksubxa` | `973973` |

For the short current seed, codes at offsets -60/-30/0/+30/+60 seconds are `269065`, `435136`, `649638`, `730930`, `987293`. The accepted drift covers only the centre three, subject to last-use checks. ROTP returns the step start timestamp; the helper exposes its integer step number.

With current code and last-use offsets -31/-30/-29, ROTP returns `1699999980`, but Rodauth's separate interval predicate `last_use + 30 < database now` permits only -31. With last-use at the fixed time or +30, ROTP returns nil. A code matching the previous step is rejected if that step is not strictly after the last-use step even when the interval has elapsed.

HMAC explicitly disabled uses the stored seed directly: short raw code `541083`, long raw code `532659`. Rotation is attempted only when a current HMAC secret and a distinct old secret are configured.

A bounded synthetic search found code `975469` at Unix time `1709698170` matching more than one accepted step. ROTP selects the latest: timestamp `1709698200`, step `56989940`. The regression test retains this distinction from totp-rs's default first-match scanning.

The exact first oracle Ruby command body is reproducible inside the inline task's literal command:

```ruby
require "openssl"
gem "rodauth", "2.48.0"
gem "rotp", "6.3.0"
require "rodauth"
require "rodauth/features/otp"
require "rotp"
oracle = Object.new.extend(Rodauth::Otp)
synthetic_secrets = {current: "synthetic-rails-secret-key-base-for-compatibility", old: "synthetic-old-rails-secret-key-base"}
now = 1_700_000_000
["abcdefghijklmnop", "abcdefghijklmnopqrstuvwxyz234567"].each do |seed|
  synthetic_secrets.each do |generation, secret|
    oracle.define_singleton_method(:compute_raw_hmac) { |data| OpenSSL::HMAC.digest("SHA256", secret, data) }
    derived = oracle.send(:otp_hmac_secret, seed)
    otp = ROTP::TOTP.new(derived)
    codes = [-60, -30, 0, 30, 60].map { |offset| [offset, otp.at(now + offset)] }
    puts "seed=#{seed} generation=#{generation} derived=#{derived} codes=#{codes.inspect}"
    [-31, -30, -29, 0, 30].each do |offset|
      last = now + offset
      matched = otp.verify(otp.at(now), drift_behind: 30, drift_ahead: 30, after: last, at: Time.at(now))
      interval_allowed = last + 30 < now
      puts "last_use_offset=#{offset} matched=#{matched.inspect} interval_allowed=#{interval_allowed}"
    end
  end
end
```

Supplementary raw-code/whitespace/collision oracle:

```ruby
require "openssl"
gem "rodauth", "2.48.0"
gem "rotp", "6.3.0"
require "rodauth"
require "rodauth/features/otp"
require "rotp"
oracle = Object.new.extend(Rodauth::Otp)
seed = "abcdefghijklmnop"
secret = "synthetic-rails-secret-key-base-for-compatibility"
oracle.define_singleton_method(:compute_raw_hmac) { |data| OpenSSL::HMAC.digest("SHA256", secret, data) }
otp = ROTP::TOTP.new(oracle.send(:otp_hmac_secret, seed))
["abcdefghijklmnop", "abcdefghijklmnopqrstuvwxyz234567"].each { |raw| puts "raw_seed=#{raw} raw_code=#{ROTP::TOTP.new(raw).at(1_700_000_000)}" }
["649\v638", "649\f638\r"].each { |code| puts "whitespace=#{code.inspect} matched=#{otp.verify(code.gsub(/\s+/, ""), at: Time.at(1_700_000_000))}" }
start = 56_666_666
previous = otp.at(start * 30)
before_previous = nil
started = Process.clock_gettime(Process::CLOCK_MONOTONIC)
(start + 1..start + 500_000).each do |step|
  current = otp.at(step * 30)
  if current == previous || current == before_previous
    now = (step - 1) * 30
    matched = otp.verify(current, drift_behind: 30, drift_ahead: 30, after: now - 91, at: Time.at(now))
    puts "collision_code=#{current} now=#{now} matched=#{matched} step=#{matched / 30}"
    break
  end
  before_previous = previous
  previous = current
  if Process.clock_gettime(Process::CLOCK_MONOTONIC) - started > 30
    puts "collision_search_bound_reached"
    break
  end
end
```

The supplementary command was initially run with whitespace passed directly to ROTP, which rejects it; Rodauth performs `gsub(/\s+/, '')` before ROTP. A subsequent focused oracle confirmed vertical tab/form feed become `649638` and match timestamp `1699999980`; non-breaking space remains unchanged and fails. The reproduction above includes that preprocessing. Tests preserve Ruby ASCII whitespace and reject Unicode lookalikes.

## Implementation boundary

`rust/api/src/auth_compatibility.rs` derives exactly the legacy modulo-32 encoding of HMAC-SHA256 over decoded seed bytes. It validates stored lowercase Base32 seeds of length 16 or 32 and rejects empty configured HMAC inputs. It verifies SHA1 six-digit OTPs with explicit current/old secrets, one step of drift and the recorded integer-time interval/after predicates.

Sixteen-character legacy seeds decode to 80 bits. totp-rs 6.0.0's regular `Builder::build` rejects secrets below 128 bits. This import-only helper therefore uses `build_noncompliant` after explicit seed validation and fixed valid algorithm/digits/step settings; it does not generate or enrol new short secrets. [Published builder source](https://docs.rs/crate/totp-rs/6.0.0/source/src/builder.rs).

No production comments were added. The coordinator owns module registration, dependency pin and lockfile updates. Dependency: exact totp-rs 6.0.0, MIT, declared Rust 1.88.

## Verification and remaining gates

RED was recorded before creating production source: the focused task failed with Rust E0583 for the missing `auth_compatibility` module. The first GREEN attempt was temporarily blocked by independently owned web modules registered before their source files landed. Once those files existed, ten initial tests passed. The final `rtk proxy task api:test TEST_FILE=auth_compatibility` run passed all 12 tests, including after the Clippy refactor. Owned-file formatting completed through task wrappers. `rtk proxy task docs:build` passed after retrying with access to the existing uv cache; `git diff --check` passed. `rtk proxy task openspec:validate` passed all 24 items.

Clippy initially found a needless borrow in the owned helper; it was corrected and the 12 tests stayed green. A subsequent run was temporarily blocked by separately owned functions awaiting route registration; no warning suppression was added. After the coordinator integrated those routes, whole-API Clippy passed and the API tests passed: 26 unit tests plus all 12 authentication compatibility tests. The [independent review](run-20261001-review.md#otp-candidate-review-at-0850-utc) accepted this bounded helper. These final outcomes were reported by the coordinator. OpenSpec tasks 1.1–1.6 are complete; all passkey, integrated factor, OIDC, lifecycle and cutover tasks remain open.

The pure helper does not establish database compatibility by itself. Integration must compare the interval against full-precision database timestamps, retain atomic last-use/failure updates and row locking, consume recovery codes once, rotate sessions and store trusted factor assurance. Rodauth writes actual database time rather than the matched OTP counter; this helper must not be described as a complete one-use counter store. Concurrent, fractional-time and future-drift reuse behaviour need explicit integration acceptance against the accepted Rails policy.

WebAuthn import remains a separate gate: retain 64-byte handles, IDs, encoded COSE public keys and counters; prove UV, replay, origin/RP and missing metadata handling before enabling existing-factor login. OIDC, lifecycle, browser/native factor completion and real provider/device acceptance remain unimplemented.
