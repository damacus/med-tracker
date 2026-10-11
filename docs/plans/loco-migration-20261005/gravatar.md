# Gravatar parity

Gravatar is core migration scope, confirmed by the owner on 11 October 2026.
Retain Rails' account opt-in: the depicted person's linked account controls
disclosure, not the person viewing the page. Uploaded photos take priority;
missing or failed Gravatar images retain local initials. Disabling the preference
removes external image URLs on the next render and immediately after saving.

The photo accordion explains that enabling Gravatar shares an email hash and
the viewer's IP address with Gravatar. Requests omit the page referrer. No
profile lookup, tracking script or server-side image download is needed.

## Library decision

The Rails component uses `Digest::MD5`, not a Gravatar gem. The Rust
[`gravatar` crate](https://docs.rs/gravatar/0.2.0/gravatar/) also uses MD5.
[`gravatar_api`](https://github.com/cathive/gravatar-api-rs) 0.3.0 offers
SHA-256 image URLs, but bundles a blocking/network client through Reqwest 0.12
and SHA-2 0.9 without a URL-only feature. Its optional query parameters also
assume that size is present when choosing the first query separator.

Use the app's existing `sha2` and `url` libraries for this small integration.
Follow [Gravatar's current identifier contract](https://docs.gravatar.com/rest/hash/):
trim and lowercase the email, then SHA-256. Request HTTPS images with an explicit
size, G rating and `d=404`. No hashing primitive is implemented in application
code. A known official hash vector and browser round trips cover the boundary.

The email precedence remains person email, legacy linked user email, then
linked account email. A person without a linked account cannot opt in through
someone else's account. Existing person access checks remain authoritative.

The independent profile source review tracked by #2502 is a separate gate.
