# Run MedTracker locally

Use this guide to run MedTracker on your own computer for local evaluation.
This development setup is not a production deployment.

## Before you begin

Run the commands in a terminal:

- On Windows, open PowerShell or Command Prompt from the Start menu.
- On macOS, open Terminal from **Applications > Utilities**.

### What you'll need

Install these tools:

1. [Docker Desktop](https://www.docker.com/products/docker-desktop/)
2. [Git](https://git-scm.com/downloads)
3. [Task](https://taskfile.dev/installation/)
4. [Portless](https://portless.sh/)

Install Portless and trust its local HTTPS certificate:

```shell
npm install -g portless
portless trust
```

---

## Download MedTracker

Clone the repository and enter its directory:

```bash
git clone https://github.com/damacus/med-tracker.git
cd med-tracker
```

## Start MedTracker

> **Local evaluation only:** `task dev:portless` starts the development stack.
> Do not run this stack on a public or shared network, and do not use it as a
> production server for real medication or person records. For a reachable
> server, use a production deployment and create your first administrator
> through the bootstrap/invitation flow instead of loading development
> fixtures.

```fish
task dev:portless
```

The first start can take a few minutes while Docker downloads and builds the
required images.

## Add example data

You can load local example people and medications. The fixtures include sample
accounts with known passwords. Only run this command on a private development
machine:

```fish
task dev:seed
```

## Open MedTracker

Open [https://med-tracker.localhost](https://med-tracker.localhost) in a web
browser.

Sign in with a local demo account from the seed output, or use the account your
administrator invited for a production deployment. If you loaded fixture data,
change or remove any sample accounts before exposing the app beyond your own
computer.

---

## Next steps

- [Add your first medicine](families/adding-first-medicine.md)
- [Record a dose](families/taking-first-dose.md)
