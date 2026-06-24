# Self-Hosting Setup

This guide walks through running MedTracker on your own computer for local evaluation only.

## 🏁 Before you begin

If you've never used your computer's "Terminal" before, don't worry! Here's how to find it:

- **Windows:** Search for "PowerShell" or "Command Prompt" in your Start menu.
- **Mac:** Open "Finder", go to "Applications" → "Utilities", and double-click **Terminal**.

### What you'll need

You'll need to install these three tools before we start. They are the "engine" that runs MedTracker:

1. **[Docker Desktop](https://www.docker.com/products/docker-desktop/):** download and install the version for your computer.
2. **[Git](https://git-scm.com/downloads):** follow the instructions for your computer.
3. **[Task](https://taskfile.dev/installation/):** follow the instructions for your computer (e.g., `brew install go-task/tap/go-task` on Mac).

---

## 1. Download MedTracker

Open your Terminal and paste these commands (one line at a time):

```bash
git clone https://github.com/damacus/med-tracker.git
cd med-tracker
```

## 2. Start the App

> **Local evaluation only:** `task dev:up` starts the development stack. Do not run
> this stack on a public or shared network, and do not use it as a production
> server for real medication or person records. For a reachable server, use a
> production deployment and create your first administrator through the
> bootstrap/invitation flow instead of loading development fixtures.

Now, let's start the app's services:

```bash
task dev:up
```

*This might take a few minutes the first time as it downloads the necessary parts. If it asks for permission to access your files or network, click **Allow**.*

## 3. Add Some Example Data

To make it easier to see how it works, you can add local-only "dummy" data
(like example people and medicines). These fixtures include sample accounts and
known passwords, so only run this command on a private development machine:

```bash
task dev:seed
```

## 4. Open MedTracker in Your Browser

Now, open your web browser (like Chrome or Safari) and go to:

👉 **[http://localhost:3000](http://localhost:3000)**

Sign in with a local demo account from the seed output, or use the account your
administrator invited for a production deployment. If you loaded fixture data,
change or remove any sample accounts before exposing the app beyond your own
computer.

---

## What's next?

Now that the app is running, you can:

- [**Add your first medicine**](families/adding-first-medicine.md)
- [**Record a dose**](families/taking-first-dose.md)
