# Self-hosting MedTracker

The 0.6 application uses Rust/Loco and PostgreSQL 18. Read the
[release notes](releases/0.6.md) before installing it. Start with a new empty
database and fresh accounts; existing Rails data, passwords, passkeys and
sessions are not imported.

## Local evaluation

Follow the [technical quick start](quick-start.md) to install the selected
tools, prepare a private development database and start the Rust server.
Synthetic fixtures are for local evaluation only. Keep them off public/shared
networks and do not enter real medication records into a fixture installation.

## A reachable installation

Operators should use the [fresh Rust rollout guide](operations/loco-cutover-runbook.md)
and [Loco image guide](operations/loco-images.md). Configure a dedicated database,
restricted runtime credentials, persistent secrets, private object storage and
the public HTTPS origin. Run the server, workers and a single scheduler through
the deployment's existing controller.

The Loco image workflow can publish tested images for both supported Linux
architectures. It is separate from the Rails publisher. Publishing an image
does not deploy an application, and these notes do not claim a production
rollout has happened.

Do not reset or repurpose an existing installation's database. Keep retained
Rails data and secrets separate. Rust-created data has no reverse migration
into Rails. Follow the rollout guide's stop and recovery boundaries if a new
installation encounters a problem.

Once an operator has prepared access, household members can follow the
[family setup guide](families/quick-setup.md). MedTracker supplements an existing
medication routine; it should not be the sole source of medication reminders.
