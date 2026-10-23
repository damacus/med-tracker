# Maintaining release notes

The [0.6 overview](../releases/0.6.md) is the human-facing account of the fresh
Rust release. Keep its claims aligned with merged behaviour and the accepted
[fresh Rust decision](../adrs/0016-fresh-rust-without-legacy-compatibility.md).
An open release PR is preparation, not proof of deployment or milestone completion.

## Generated changelog and release PR

The main-branch release-please workflow reads `release-please-config.json` and
`.release-please-manifest.json`. It generates the pending `CHANGELOG.md` entry,
the release PR body and the configured version update. The current configuration
still uses the Ruby strategy and `rails/lib/med_tracker/version.rb` as its version
file. That bookkeeping does not select a Rails runtime or migrate credentials.

Do not hand-edit the generated breaking note alone: a later run can restore the
old wording from commit metadata. Release-please supports a
[`BEGIN_COMMIT_OVERRIDE` block in the merged source PR](https://github.com/googleapis/release-please#how-can-i-fix-release-notes).
For the 0.6 authentication note, that source is
[PR #2476](https://github.com/damacus/med-tracker/pull/2476), not the generated
[release PR #2381](https://github.com/damacus/med-tracker/pull/2381).
Preserve unrelated release entries when correcting its override.

The breaking note must state the current installation boundary:

> Start the Rust application with a new empty PostgreSQL 18 database and fresh
> accounts. Rails passwords, passkeys, authenticator enrolments, sessions and
> recovery tokens are not imported. Existing Rails sessions cannot authenticate
> to Rust. Clinical data transfer uses portable export/import, with complete
> Rails-export compatibility tracked in #2702. There is no Rails schema adoption,
> password/passkey/session/auth-record conversion or reverse migration. Keep existing installations separate; do not reset their databases
> or assume Rails can consume Rust-created records.

There is no supported rollback from 0.6 to Rails. A full Rails export at the
transition is an operator action, not an automatic import or a claim that an
export has already happened. Any retained Rails site contains only its
pre-cutover state. Preserve data written in 0.6 with a separate Rust-side export;
returning to the old site would not carry those records back. These notes do
not promise that Rails can import that export or provide a lossless switchback.

Review the override and generated release body together before updating GitHub.
After release-please next runs, check that the generated changelog and PR body
retain the corrected boundary. Historical changelog entries remain historical;
do not rewrite already published releases to describe the new runtime.

Merging the release PR can create a tag/release and start downstream publishing
workflows. Correcting release notes does not authorise that action. Publishing
the docs source also does not itself authorise a site deployment.
