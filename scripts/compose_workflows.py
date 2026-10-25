"""Checkout-owned local Compose lifecycle; no external database configuration."""

import argparse
from functools import lru_cache
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[1]


def project(environment, root=ROOT):
    return "mt-" + hashlib.sha256(str(root.resolve()).encode()).hexdigest()[:12] + "-" + environment


def frontend_lock(root=ROOT):
    return "/root/.npm/" + project("dev", root)[:-4] + "-frontend.lock"


def cache_names(architecture, root=ROOT, lock_bytes=None, target_override=None):
    architecture = {"aarch64": "arm64", "x86_64": "amd64"}.get(architecture, architecture)
    platform = "linux-" + architecture
    checkout = project("dev", root)[:-4]
    target = checkout + "-target-" + platform
    allowed_targets = {target, project("dev", root) + "_target", project("test", root) + "_target"}
    if target_override and target_override not in allowed_targets:
        raise ValueError("Target cache must be an exact allowed target volume for this checkout")
    dockerfile = (ROOT / "Dockerfile").read_bytes()
    node_image = next(line for line in dockerfile.splitlines() if line.startswith(b"FROM node:"))
    lock_bytes = (root / "package-lock.json").read_bytes() if lock_bytes is None else lock_bytes
    node_key = hashlib.sha256(node_image + lock_bytes + platform.encode()).hexdigest()[:20]
    browser_key = hashlib.sha256(dockerfile + platform.encode()).hexdigest()[:20]
    return {
        "MT_TARGET_CACHE": target_override or target,
        "MT_CARGO_REGISTRY_CACHE": "mt-cargo-registry-v1",
        "MT_CARGO_GIT_CACHE": "mt-cargo-git-v1",
        "MT_NODE_CACHE": checkout + "-node-" + platform + "-" + node_key,
        "MT_NPM_CACHE": "mt-npm-downloads-v1",
        "MT_BROWSER_CACHE": "mt-browser-" + platform + "-" + browser_key,
    }


@lru_cache
def docker_architecture():
    return subprocess.check_output(["docker", "info", "--format", "{{.Architecture}}"], text=True).strip()


@lru_cache
def create_cache_volume(name):
    subprocess.run(["docker", "volume", "create", "--label", "io.medtracker.cache=true", name], check=True, stdout=subprocess.DEVNULL)


def compose(environment):
    return ["docker", "compose", "--env-file", "/dev/null", "-p", project(environment),
            "-f", str(ROOT / "compose.yaml"), "-f", str(ROOT / "compose.local.yaml"),
            "--profile", "fixture-storage", "--profile", environment]


def run(environment, *arguments, capture=False):
    clean = {key: value for key, value in os.environ.items()
             if not key.startswith("COMPOSE_")}
    clean["MT_ENV"] = environment
    clean["MT_PROJECT"] = project(environment)
    clean["MT_FRONTEND_LOCK"] = frontend_lock()
    caches = cache_names(docker_architecture(), target_override=os.environ.get("MT_TARGET_CACHE"))
    clean.update(caches)
    if environment != "prod" and arguments[0] in ("run", "up"):
        for name in caches.values():
            create_cache_volume(name)
    return subprocess.run(compose(environment) + list(arguments), cwd=ROOT, env=clean,
                          check=True, text=True, capture_output=capture)


def tool(environment, *arguments):
    run(environment, "run", "--rm", "--no-deps", "tool", *arguments)


def frontend(environment, *arguments):
    tool(environment, "flock", frontend_lock(), *arguments)


def services(environment):
    check_data_volumes(environment)
    run(environment, "up", "-d", "--wait", "postgres", "rustfs")
    reference = run(environment, "exec", "-T", "postgres", "psql", "-X", "-U", "medtracker", "-d", "medtracker_loco", "-tAc", "SELECT 1 FROM pg_database WHERE datname = 'medtracker_reference'", capture=True)
    if not reference.stdout.strip():
        run(environment, "exec", "-T", "postgres", "createdb", "-U", "medtracker", "medtracker_reference")
    endpoint = run(environment, "port", "rustfs", "9000", capture=True).stdout.strip()
    for bucket in ("scanner-fixture", "medtracker-fixture"):
        subprocess.run(["curl", "--fail", "--silent", "--show-error", "--aws-sigv4", "aws:amz:us-east-1:s3", "--user", "storage-smoke-access:storage-smoke-secret", "--request", "PUT", "http://" + endpoint + "/" + bucket], check=True)


def check_data_volumes(environment):
    for name in ("database", "scanner-data", "scanner-logs"):
        volume = project(environment) + "_" + name
        result = subprocess.run(["docker", "volume", "inspect", volume], text=True, capture_output=True)
        if result.returncode:
            if "no such volume" in result.stderr.lower():
                continue
            raise RuntimeError(result.stderr)
        labels = json.loads(result.stdout)[0].get("Labels") or {}
        if labels.get("io.medtracker.local-owner") != project(environment):
            raise RuntimeError("Refusing to adopt or reset unowned volume " + volume)


def application(environment, *arguments):
    if environment == "prod":
        run(environment, "run", "--rm", "--no-deps", "app", *arguments)
    else:
        tool(environment, "cargo", "run", "--locked", "--", *arguments)


def wait_ready(environment):
    service = "app" if environment == "prod" else environment + "-app"
    endpoint = run(environment, "port", service, "5150", capture=True).stdout.strip()
    origin = "http://" + endpoint
    deadline = time.monotonic() + 90
    while time.monotonic() < deadline:
        try:
            with urllib.request.urlopen(origin + "/health", timeout=2) as response:
                if response.status == 200:
                    print(project(environment) + " ready at " + origin, flush=True)
                    return
        except (urllib.error.URLError, OSError):
            pass
        time.sleep(1)
    run(environment, "logs", "--tail", "40", service)
    raise RuntimeError("Application did not become healthy at " + origin)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("environment", choices=["dev", "test", "prod"])
    parser.add_argument("action", choices=["up", "down", "restart", "logs", "ps", "build", "reset", "seed", "unit", "integration", "browser"])
    parser.add_argument("--target", default="")
    parser.add_argument("--filter", default="")
    parser.add_argument("--confirm", default="")
    args = parser.parse_args(argv)
    environment, action = args.environment, args.action
    loco = {"dev": "development", "test": "test", "prod": "production"}[environment]
    if action in ("seed", "reset") and environment == "prod":
        parser.error("production has no seed or reset operation")
    if action in ("unit", "integration", "browser") and environment != "test":
        parser.error("test commands require the test environment")
    if action == "reset":
        if args.confirm != project(environment):
            parser.error("reset requires --confirm " + project(environment))
        check_data_volumes(environment)
        run(environment, "down")
        for name in ("database", "scanner-data", "scanner-logs"):
            subprocess.run(["docker", "volume", "rm", project(environment) + "_" + name], check=True)
    elif action == "build":
        run(environment, "build", "app" if environment == "prod" else "tool")
    elif action in ("down", "logs", "ps"):
        run(environment, action, *(["--follow"] if action == "logs" else []))
    elif action == "restart":
        run(environment, "restart")
    elif action in ("unit", "integration", "browser"):
        if action != "unit":
            services(environment)
        command = ["cargo", "nextest", "run", "--locked", "--no-tests=fail", "--test-threads", "2"]
        if action == "unit":
            command += ["--lib", "--package", args.target] if args.target else ["--workspace", "--lib"]
        elif action == "browser":
            command += ["--test", args.target or "browser", "--run-ignored", "only"]
        else:
            command += ["--test", args.target] if args.target else ["--tests"]
        if args.filter:
            command.append(args.filter)
        frontend(environment, "task", "--taskfile", "Taskfiles/frontend.yml", "--dir", "/app", "prepare")
        tool(environment, *command)
        if action == "integration" and not args.target and not args.filter:
            tool(environment, "cargo", "test", "--locked", "--test", "template_concurrency", "competing_processes_prepare_once", "--", "--ignored", "--exact")
        if action == "unit" and not args.filter:
            frontend(environment, "task", "--taskfile", "Taskfiles/frontend.yml", "--dir", "/app", "test")
            tool(environment, "cargo", "test", "--locked", "--workspace", "--doc")
    elif action == "seed":
        services(environment)
        application(environment, "db", "seed", "--environment", loco)
    elif action == "up":
        services(environment)
        if environment != "prod":
            frontend(environment, "task", "--taskfile", "Taskfiles/frontend.yml", "--dir", "/app", "prepare")
        if environment == "prod":
            run(environment, "run", "--rm", "--no-deps", "-e", "DATABASE_URL=postgres://medtracker:medtracker_password@postgres/medtracker_loco", "app", "db", "migrate", "--environment", loco)
        else:
            application(environment, "db", "migrate", "--environment", loco)
        exists = run(environment, "exec", "-T", "postgres", "psql", "-X", "-U", "medtracker", "-d", "medtracker_loco", "-tAc", "SELECT 1 FROM pg_roles WHERE rolname = 'medtracker_runtime'", capture=True)
        if not exists.stdout.strip():
            run(environment, "exec", "-T", "postgres", "createuser", "-U", "medtracker", "--login", "--inherit", "--no-superuser", "--no-createdb", "--no-createrole", "--member-of=med_tracker_app", "medtracker_runtime")
            # SeaORM has no PostgreSQL role-administration API. This isolated local
            # provisioning statement contains only fixed synthetic credentials.
            run(environment, "exec", "-T", "postgres", "psql", "-X", "-v", "ON_ERROR_STOP=1", "-U", "medtracker", "-d", "medtracker_loco", "-c", "ALTER ROLE medtracker_runtime NOBYPASSRLS PASSWORD 'synthetic_local_runtime_password'")
        restricted = run(environment, "exec", "-T", "postgres", "psql", "-X", "-v", "ON_ERROR_STOP=1", "-U", "medtracker", "-d", "medtracker_loco", "-tAc", "SELECT rolcanlogin AND rolinherit AND NOT rolsuper AND NOT rolcreatedb AND NOT rolcreaterole AND NOT rolbypassrls AND pg_has_role('medtracker_runtime', 'med_tracker_app', 'MEMBER') FROM pg_roles WHERE rolname = 'medtracker_runtime'", capture=True)
        if restricted.stdout.strip() != "t":
            raise RuntimeError("Expected a restricted runtime identity; refusing to alter an existing role")
        run(environment, "up", "-d", *(["app", "scheduler"] if environment == "prod" else [environment + "-app", environment + "-scheduler"]))
        wait_ready(environment)


if __name__ == "__main__":
    try:
        main()
    except subprocess.CalledProcessError as error:
        sys.exit(error.returncode)
    except (RuntimeError, ValueError) as error:
        sys.exit(str(error))
