"""Checkout-owned local Compose lifecycle; no external database configuration."""

import argparse
from functools import lru_cache
import hashlib
import json
import os
import re
import shutil
import socket
from pathlib import Path
import subprocess
import sys
import time
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[1]


def project(environment, root=ROOT):
    return "mt-" + hashlib.sha256(str(root.resolve()).encode()).hexdigest()[:12] + "-" + environment


def public_origin(environment):
    if os.environ.get("PORTLESS") == "0":
        return "http://localhost"
    port = os.environ.get("PORTLESS_PORT", "443")
    return "https://" + project(environment) + ".localhost" + (":" + port if port != "443" else "")


def portless(*arguments):
    executable = shutil.which("portless")
    if not executable and shutil.which("mise"):
        resolved = subprocess.run(["mise", "which", "portless"], cwd=ROOT, text=True, capture_output=True)
        if resolved.returncode == 0:
            executable = resolved.stdout.strip()
    if not executable:
        raise RuntimeError("Portless is missing; run mise install npm:portless, then portless doctor. Use PORTLESS=0 to bypass.")
    clean = {**os.environ, "PORTLESS_SYNC_HOSTS": "0", "PORTLESS_LAN": "0", "NO_COLOR": "1"}
    return subprocess.run([executable, *arguments], env=clean, check=True, text=True, capture_output=True).stdout.strip()


def localhost_proxy():
    diagnostics = portless("doctor")
    if not re.search(r"^Mode:\s+HTTPS,\s+\.localhost$", diagnostics, re.MULTILINE):
        raise RuntimeError("Portless aliases require an HTTPS proxy configured exclusively for .localhost. Run portless doctor/setup, or use PORTLESS=0.")


def alias_port(environment):
    routes = portless("list")
    origin = public_origin(environment)
    match = re.search(re.escape(origin) + r"\s+->\s+localhost:(\d+)\s+\(alias\)", routes)
    if match:
        return int(match.group(1))
    if origin in routes:
        raise RuntimeError("Portless route is not an owned static alias: " + origin)
    return None


def app_binding(environment):
    service = "app" if environment == "prod" else environment + "-app"
    if not run(environment, "ps", "-q", service, capture=True).stdout.strip():
        return None
    return run(environment, "port", service, "5150", capture=True).stdout.strip()


def bypass_endpoint(environment):
    existing = app_binding(environment)
    if existing:
        port = int(existing.rsplit(":", 1)[1])
    else:
        with socket.socket() as listener:
            listener.bind(("127.0.0.1", 0))
            port = listener.getsockname()[1]
    return "http://127.0.0.1:" + str(port), port


def portless_preflight(environment):
    if os.environ.get("PORTLESS") == "0":
        return None
    localhost_proxy()
    origin = public_origin(environment)
    if portless("get", project(environment), "--no-worktree") != origin:
        raise RuntimeError("Portless HTTPS proxy is not configured/running at the expected .localhost origin. Run portless doctor/setup; set PORTLESS_PORT if needed, or use PORTLESS=0.")
    hostname = project(environment) + ".localhost"
    port = os.environ.get("PORTLESS_PORT", "443")
    probe = subprocess.run(["curl", "--silent", "--show-error", "--head", "--max-time", "3", "--resolve", hostname + ":" + port + ":127.0.0.1", origin], text=True, capture_output=True)
    if probe.returncode or "x-portless: 1" not in probe.stdout.lower():
        raise RuntimeError("Trusted Portless HTTPS proxy is unavailable. Run portless doctor and complete proxy/trust setup before up, or use PORTLESS=0. " + probe.stderr.strip())
    binding = app_binding(environment)
    previous = int(binding.rsplit(":", 1)[1]) if binding else None
    existing = alias_port(environment)
    if existing is not None and existing != previous:
        raise RuntimeError("Portless alias does not match the owned Compose app; refusing to overwrite " + origin)
    return previous


def register_alias(environment, binding, previous):
    if os.environ.get("PORTLESS") == "0":
        print(project(environment) + " ready at http://" + binding, flush=True)
        return
    localhost_proxy()
    current = int(binding.rsplit(":", 1)[1])
    existing = alias_port(environment)
    if existing is not None and existing not in (previous, current):
        raise RuntimeError("Portless alias is not owned by this Compose app; refusing to overwrite it")
    portless("alias", project(environment), str(current))
    origin = public_origin(environment)
    hostname = project(environment) + ".localhost"
    subprocess.run(["curl", "--fail", "--silent", "--show-error", "--max-time", "3", "--retry", "3", "--retry-delay", "1", "--retry-all-errors", "--retry-max-time", "5", "--resolve", hostname + ":" + os.environ.get("PORTLESS_PORT", "443") + ":127.0.0.1", origin + "/health"], check=True, stdout=subprocess.DEVNULL)
    print(project(environment) + " ready at " + origin, flush=True)


def remove_alias(environment):
    if os.environ.get("PORTLESS") == "0":
        return
    localhost_proxy()
    existing = alias_port(environment)
    if existing is None:
        return
    binding = app_binding(environment)
    if not binding or existing != int(binding.rsplit(":", 1)[1]):
        raise RuntimeError("Alias no longer matches the owned app; leave it untouched. Inspect it before explicit cleanup: portless alias --remove " + project(environment))
    portless("alias", "--remove", project(environment))


def cache_names(architecture, root=ROOT):
    architecture = {"aarch64": "arm64", "x86_64": "amd64"}.get(architecture, architecture)
    platform = "linux-" + architecture
    checkout = project("dev", root)[:-4]
    return {
        "MT_TARGET_CACHE": checkout + "-target-" + platform,
        "MT_CARGO_REGISTRY_CACHE": "mt-cargo-registry-v1",
        "MT_CARGO_GIT_CACHE": "mt-cargo-git-v1",
        "MT_BROWSER_CACHE": "mt-browser-" + platform + "-ubuntu26-v1",
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


def run(environment, *arguments, capture=False, origin=None, published_port=None):
    clean = {key: value for key, value in os.environ.items()
             if not key.startswith("COMPOSE_")}
    clean["MT_PUBLIC_ORIGIN"] = origin or ("" if os.environ.get("PORTLESS") == "0" else public_origin(environment))
    clean["MT_HTTP_PORT"] = str(published_port or 0)
    clean["MT_COOKIE_SECURE"] = "false" if os.environ.get("PORTLESS") == "0" else "true"
    clean["MT_ENV"] = environment
    clean["MT_PROJECT"] = project(environment)
    clean["MT_TOOL_TARGET"] = "test" if environment == "test" else "development"
    caches = cache_names(docker_architecture())
    clean.update(caches)
    if environment != "prod" and arguments[0] in ("run", "up"):
        for name in caches.values():
            create_cache_volume(name)
    return subprocess.run(compose(environment) + list(arguments), cwd=ROOT, env=clean,
                          check=True, text=True, capture_output=capture)


def tool(environment, *arguments):
    run(environment, "run", "--rm", "--no-deps", "dev-tool" if environment == "dev" else "tool", *arguments)


def build_tools(environment):
    run(environment, "build", "dev-tool" if environment == "dev" else "tool")


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
    elif environment == "dev":
        tool(environment, "/app/run/bin/med-tracker", *arguments)
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
                    return endpoint
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
        remove_alias(environment)
        run(environment, "down")
        for name in ("database", "scanner-data", "scanner-logs"):
            subprocess.run(["docker", "volume", "rm", project(environment) + "_" + name], check=True)
    elif action == "build":
        if environment == "prod":
            run(environment, "build", "app")
        else:
            build_tools(environment)
    elif action in ("down", "logs", "ps"):
        if action == "down":
            remove_alias(environment)
        run(environment, action, *(["--follow"] if action == "logs" else []))
    elif action == "restart":
        run(environment, "restart")
    elif action in ("unit", "integration", "browser"):
        build_tools(environment)
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
        tool(environment, *command)
        if action == "integration" and not args.target and not args.filter:
            tool(environment, "cargo", "test", "--locked", "--test", "template_concurrency", "competing_processes_prepare_once", "--", "--ignored", "--exact")
        if action == "unit" and not args.filter:
            tool(environment, "task", "--taskfile", "Taskfiles/frontend.yml", "--dir", "/app", "test")
            tool(environment, "cargo", "test", "--locked", "--workspace", "--doc")
    elif action == "seed":
        if environment == "test":
            build_tools(environment)
        services(environment)
        application(environment, "db", "seed", "--environment", loco)
    elif action == "up":
        previous = portless_preflight(environment)
        if environment != "prod":
            build_tools(environment)
        if environment == "dev":
            run(environment, "stop", "dev-frontend", "dev-builder")
            tool(environment, "task", "--taskfile", "Taskfiles/frontend.yml", "--dir", "/app", "build")
            tool(environment, "task", "--dir", "/app", "dev-build")
        services(environment)
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
        origin, published_port = (None, None)
        if os.environ.get("PORTLESS") == "0" and environment != "prod":
            origin, published_port = bypass_endpoint(environment)
        run(environment, "up", "-d", *(["app", "scheduler"] if environment == "prod" else (["dev-frontend", "dev-builder", "dev-app", "dev-scheduler"] if environment == "dev" else ["test-app", "test-scheduler"])), origin=origin, published_port=published_port)
        binding = wait_ready(environment)
        register_alias(environment, binding, previous)


if __name__ == "__main__":
    try:
        main()
    except subprocess.CalledProcessError as error:
        sys.exit(error.returncode)
    except (RuntimeError, ValueError) as error:
        sys.exit(str(error))
