"""Check the release image with disposable PostgreSQL and synthetic credentials."""

import json
import os
import subprocess
import tarfile
import time
import unittest
import urllib.error
import urllib.request
import uuid


class LocoImageTest(unittest.TestCase):
    image = os.environ.get("LOCO_IMAGE", "medtracker-loco:local")

    def docker(self, *arguments):
        return subprocess.check_output(["docker", *arguments], text=True, timeout=180)

    def test_production_boot_with_restricted_database_identity(self):
        prefix = "loco-image-" + uuid.uuid4().hex
        database, server, worker = (
            prefix + suffix for suffix in ("-db", "-server", "-worker")
        )
        containers = []
        self.docker("network", "create", prefix)
        try:
            self.docker(
                "run",
                "--detach",
                "--name",
                database,
                "--network",
                prefix,
                "--tmpfs",
                "/var/lib/postgresql",
                "--env",
                "POSTGRES_HOST_AUTH_METHOD=trust",
                "--env",
                "POSTGRES_DB=medtracker_image",
                "postgres:18-alpine@sha256:898c60cb390d1085bc97bdd6a1ae968757421df394b071848b52b1c09c89d6a5",
            )
            containers.append(database)
            for _ in range(60):
                ready = subprocess.run(
                    [
                        "docker",
                        "exec",
                        database,
                        "pg_isready",
                        "-h",
                        "127.0.0.1",
                        "-U",
                        "postgres",
                    ],
                    check=False,
                    capture_output=True,
                    timeout=10,
                )
                if ready.returncode == 0:
                    break
                time.sleep(1)
            else:
                self.fail("Disposable PostgreSQL did not become ready")
            common = [
                "--network",
                prefix,
                "--read-only",
                "--cap-drop=ALL",
                "--security-opt=no-new-privileges=true",
                "--tmpfs",
                "/tmp:uid=65532,gid=65532",
                "--env",
                "MEDTRACKER_PUBLIC_HOST=https://medtracker.invalid",
                "--env",
                "MEDTRACKER_SESSION_KEY=" + "BwcH" * 21 + "Bw==",
                "--env",
                "AUTH_SESSION_SECRET=synthetic-image-test-secret-at-least-32-characters",
            ]
            self.docker(
                "run",
                "--rm",
                *common,
                "--env",
                f"DATABASE_URL=postgres://postgres@{database}/medtracker_image",
                self.image,
                "db",
                "migrate",
                "--environment",
                "production",
            )
            self.docker(
                "exec",
                database,
                "createuser",
                "-U",
                "postgres",
                "--login",
                "--inherit",
                "--no-superuser",
                "--no-createdb",
                "--no-createrole",
                "--member-of=med_tracker_app",
                "image_runtime",
            )
            runtime = [
                *common,
                "--env",
                f"DATABASE_URL=postgres://image_runtime@{database}/medtracker_image",
            ]
            self.docker(
                "run",
                "--rm",
                *runtime,
                self.image,
                "scheduler",
                "--list",
                "--environment",
                "production",
            )
            for name, arguments in [
                (server, []),
                (worker, ["start", "--worker", "--environment", "production"]),
            ]:
                self.docker(
                    "run",
                    "--detach",
                    "--name",
                    name,
                    *runtime,
                    *(["--publish", "127.0.0.1::5150"] if name == server else []),
                    self.image,
                    *arguments,
                )
                containers.append(name)
            port = self.docker("port", server, "5150").strip().rsplit(":", 1)[1]
            origin = f"http://127.0.0.1:{port}"
            for _ in range(60):
                try:
                    with urllib.request.urlopen(
                        origin + "/health", timeout=2
                    ) as response:
                        self.assertEqual(response.status, 200)
                    break
                except (urllib.error.URLError, TimeoutError):
                    time.sleep(1)
            else:
                self.fail(
                    "Production server did not become healthy: "
                    + self.docker("logs", server)
                )
            with urllib.request.urlopen(
                origin + "/static/css/compiled.css", timeout=5
            ) as response:
                self.assertGreater(len(response.read()), 1000)
            with urllib.request.urlopen(origin + "/login", timeout=5) as response:
                self.assertEqual(response.status, 200)
                self.assertIn(b"<form", response.read())
            for name in (server, worker):
                self.assertEqual(
                    self.docker(
                        "inspect", "--format", "{{.State.Running}}", name
                    ).strip(),
                    "true",
                    self.docker("logs", name),
                )
        finally:
            for container in reversed(containers):
                self.docker("rm", "--force", "--volumes", container)
            self.docker("network", "rm", prefix)

    def test_commands_load_shared_libraries(self):
        for arguments in [
            ("--help",),
            ("start", "--help"),
            ("db", "--help"),
            ("scheduler", "--help"),
        ]:
            with self.subTest(arguments=arguments):
                output = self.docker(
                    "run",
                    "--rm",
                    "--pull=never",
                    "--network=none",
                    "--read-only",
                    "--cap-drop=ALL",
                    "--security-opt=no-new-privileges=true",
                    self.image,
                    *arguments,
                )
                self.assertIn("Usage:", output)

    def test_unprivileged_runtime_and_assets(self):
        metadata = json.loads(self.docker("image", "inspect", self.image))[0]["Config"]
        self.assertEqual(metadata["User"], "65532:65532")
        self.assertEqual(metadata["WorkingDir"], "/app")
        self.assertEqual(metadata["Entrypoint"], ["/app/med-tracker"])
        self.assertEqual(metadata["Cmd"], ["start", "--environment", "production"])
        container = self.docker("create", "--pull=never", self.image).strip()
        try:
            with subprocess.Popen(
                ["docker", "export", container], stdout=subprocess.PIPE
            ) as process:
                with tarfile.open(fileobj=process.stdout, mode="r|") as archive:
                    files = {member.name: member.size for member in archive}
                self.assertEqual(process.wait(), 0)
            for path in [
                "app/config/production.yaml",
                "app/assets/static/css/compiled.css",
                "app/assets/static/js/html5-qrcode.min.js",
                "app/assets/reports/fonts/NotoSans-Regular.ttf",
                "etc/ssl/certs/ca-certificates.crt",
                "usr/share/zoneinfo/Europe/London",
            ]:
                with self.subTest(path=path):
                    self.assertGreater(files.get(path, 0), 0)
            self.assertTrue(any(path.startswith("app/assets/views/") for path in files))
            for path in [
                "bin/sh",
                "usr/bin/cargo",
                "usr/bin/node",
                "usr/local/cargo/bin/cargo",
                "usr/local/bin/node",
                "root/.cargo/bin/cargo",
                "app/.env",
                "app/.git/config",
                "app/rails/config/ai_medication_sources.yml",
            ]:
                self.assertNotIn(path, files)
        finally:
            self.docker("rm", container)


if __name__ == "__main__":
    unittest.main()
