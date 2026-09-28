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
Inspect the documented meaning of person_id on update before choosing its
behaviour; the source currently checks its visibility but ignores reassignment.

Pause/resume and period history follow as a separate medication batch.
Sol owns production, specification clarification and shared runner wiring;
Luna owns the focused black-box test file. Compile and demonstrate initial
RED before production changes, then expand tests in parallel. Independent
review, isolated acceptance and publication precede completion credit.
