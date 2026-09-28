# Schedule writes

After person-medication assignment writes, implement createSchedule,
updateSchedule and replaceSchedule. Review and verify the existing
listSchedules and getSchedule operations alongside them.

Use the authoritative OpenAPI and Rails SchedulesController, Schedule and
SchedulePolicy. Existing Rust schedule entities, reads and serializer are
reusable; schedules.rs contains source behaviour tests, not credited Rust
acceptance. Reuse reviewed authentication, audit, sync and idempotency helpers.

POST returns 201 and ETag. PATCH and PUT merge supplied fields, accept an
omitted If-Match and reject a stale value with 409. Require person management
authority and visible medication/source dosage links. Validate dosage-source
matching, dates, recurrence type and recurrence configuration. Verify numeric
and portable links, private read visibility, strict response fields, rejected
write nonmutation, version changes, correlated audit and sync effects.

Do not reproduce known Rails defects: silent fractional interval truncation,
unknown recurrence types returning 500, invalid times being accepted, or
inaccessible dosage options being disclosed through a validation response.
Resolve and document the storage/type constraints before deriving tests;
reject unrepresentable decimal amounts rather than silently rounding.
The person association is immutable on update: the current numeric or portable
identifier is accepted, a different visible person returns 422, and hidden or
foreign identifiers return 404. Document this instead of silently ignoring an
attempted reassignment. Minimum hours retain decimal-string transport but must
represent an exact positive whole number fitting the integer storage column;
fractional values return 422 rather than being truncated. No database type
migration is included in this batch.

Pause/resume and period history follow as a separate medication batch.
Sol owns production, specification clarification and shared runner wiring;
Luna owns the focused black-box test file. Compile and demonstrate initial
RED before production changes, then expand tests in parallel. Independent
review, isolated acceptance and publication precede completion credit.

Unchanged updates preserve timestamps and ETags and create no version or sync
event, while retaining request audits and idempotent responses. This intentionally
avoids the spurious unchanged-save events possible in Rails SyncTrackable.
