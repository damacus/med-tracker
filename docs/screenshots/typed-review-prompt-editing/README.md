# Typed review-prompt editing reference

The `reference/` captures were taken from unchanged production source at
`7eed417359eef9d0347b96ede9fa8ed410a46179`. The sibling captures use the
typed editing candidate. Both runs used the browser journey's synthetic review
evidence and fixture household, English locale, and full-page desktop
(1440 × 1000) and mobile (390 × 844) viewports. Each capture waited for
`body[data-navigation-ready]`.

The four states show the review index, open edit form, invalid practitioner
review submission, and successful saved review. All eight corresponding PNG
pairs are byte-identical (SHA-256); there are no visual or clock differences in
these captures. The saved state was also checked against the persisted prompt.

The separate clear-date regression has no screenshot: on the unchanged source,
the date cleared in storage but `updated_at` and the derived ETag did not
advance. On the typed candidate, the same browser journey advances the version.
