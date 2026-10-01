# Rails CI 2354 bounded diagnosis

1 October 2026. Read-only source/log review; Ruby discovery, review, Rails and
debugging references loaded. No reproduction, code changes, waits/retries or
runtime were performed by the reviewer. The existing verifier owns focused
reproduction when idle; E final review takes precedence.

## Confirmed evidence

The saved CI log `/private/tmp/rust-dosage-rails-system-ci.log` hashes to
`fc4173f4c551de6ef905565985de051d2b4172e937e955113a2716c32226e537`.
Its failure at lines 1638–1648 and final result at 1802–1818 show one failure
among 122 browser examples in shard 1. The failed example is
`spec/system/medications/refill_inventory_spec.rb:66`; line 77 expects
“Scanned stock added successfully.” The captured page is the Paracetamol detail
view with quantity 30, but neither success notice appears. The subsequent DB
quantity assertion at line 78 was not reached. Do not report that assertion as
passed, although the displayed updated stock supports a successful write.
The coordinator reports Rails source is unchanged from the parent; this review
does not attribute the failure to the Rust change.

The log provides selected example IDs (line 1545), not a random seed or HTTP
request trace. There is no request-by-request timing, redirected response body,
flash state or cookie trace sufficient to identify the actual cause. CI used
Ruby 4.0.7 and Rails 8.1.4, as recorded in the log; do not substitute the older
versions in repository prose when reproducing the job.

## Actual request and notice paths

1. Inventory renders the scan dialog in its header
   (`app/components/medications/index_view.rb:77`), outside the inventory Turbo
   frame. The dialog contains a normal form_with POST to scan_restock
   (`inventory_scan_modal.rb:56`). It does not target a modal Turbo frame.
2. The barcode field binds both input and change to lookup
   (`inventory_scan_modal.rb:80`). `inventory_scan_controller.js:15` launches an
   authenticated same-origin JSON fetch for each event. Its increasing sequence
   only suppresses stale response DOM processing (`:39`); it does not abort the
   request or provide disconnect/submit cancellation. A lookup can therefore
   overlap the stock POST and subsequent navigation.
3. `app/controllers/medications_controller.rb:152` resolves and authorises the
   medication, calls the restock service, and on success redirects to detail
   with the expected notice (`:217`). scan_restock_match at `:162` returns JSON;
   it neither renders a layout nor explicitly reads/discards flash.
4. Detail show passes flash[:notice] into ShowView (`:37`). ShowView renders a
   persistent inline success alert (`show_view.rb:20` and `:43`) with no flash
   controller. Separately the application layout renders global flash
   (`app/views/layouts/application.html.erb:70`); its notice wrapper uses a 3000ms
   timer (`app/components/layouts/flash.rb:28`). The flash controller fades then
   removes only its own element after a further 300ms (`flash_controller.js:23`).

The three-second timer alone is consequently a weak explanation: it does not
remove the persistent ShowView notice. Missing both copies suggests the rendered
detail response lacked the notice, or the browser later replaced that response.
No inspected dialog/session-expiry code deliberately replaces the detail with
a notice-free page after successful submission.

## Cause candidates, not established causes

An overlapping lookup/request-cookie race is plausible, but “the JSON GET
consumes flash” is not demonstrated. Context7 and the linked
[Rails Flash source](https://github.com/rails/rails/blob/3989ebf3473d71e4ceca28154b0b57b5bf22db24/actionpack/lib/action_dispatch/middleware/flash.rb#L68)
show flash is initialised on access and serialised/discarded through
commit_flash when flash_hash exists. Merely sending an authenticated JSON GET
does not prove that path ran. Its controller does not explicitly access flash.
The exact CI patch's flash source could not be fetched, so this is a framework
mechanism reference, not version-specific execution proof.

[Rails 8.1.4 CookieStore source](https://github.com/rails/rails/blob/v8.1.4/actionpack/lib/action_dispatch/middleware/session/cookie_store.rb#L71)
uses request cookie data and writes session data through an encrypted cookie.
A late response overwriting the redirect's cookie is a separate candidate, which
requires observing Set-Cookie timing and whether the response actually writes a
session. A duplicate detail GET, Turbo replacement/restoration, or other
authenticated background response could also explain a notice-free final DOM.
The saved CI log proves none of those occurred. Timer-only failure, deliberate
flash consumption and a confirmed stock-write defect are all premature rulings.

## Smallest useful reproduction

Use the existing verifier and repository wrapper for the exact example:
`task test TEST_FILE=spec/system/medications/refill_inventory_spec.rb:66`.
Taskfile.yml:112 forwards the selection to RSpec in the existing test boundary.
Start with unchanged assertions and source. Keep the expected notice and
persisted quantity assertions; do not replace them with weaker visibility or
stock-only success checks. If isolated execution passes, preserve the CI shard's
selected predecessor order before attributing it to arbitrary flakiness.

Capture request/response order for the barcode JSON requests, scan POST,
redirected detail GET and any second detail GET; record status, final URL,
response notice presence, DOM inline/global notice presence, Turbo navigation
events and Set-Cookie occurrence/timing. Do not print raw cookies, tokens or
patient data. Compare response HTML with the final DOM: absent in the first
detail body points to request/session handling; present in body but absent in
DOM points to a later browser replacement/removal.

If a lookup race is indicated, use a deterministic test-only gate on that
specific request to compare completion before submit versus overlap after the
POST redirect; avoid sleeps or blanket retries. Retain realistic JSON responses
and the real stock mutation. Existing request tests in
`spec/requests/medications_refill_spec.rb:77` cover write/redirect but do not
exercise this overlapping browser request lifecycle or assert this notice.
No production fix is recommended until the focused evidence selects a cause.
