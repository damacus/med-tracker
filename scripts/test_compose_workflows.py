import importlib.util
import pathlib
import unittest
from unittest.mock import patch
import subprocess
import http.client
import json
import os
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("workflows", ROOT / "scripts/compose_workflows.py")
workflow = importlib.util.module_from_spec(spec)
spec.loader.exec_module(workflow)

class Workflows(unittest.TestCase):
    def test_test_snapshot_has_no_source_bind_mounts_or_watchers(self):
        configuration = self.configuration("test")
        for name in ("tool", "test-app", "test-scheduler"):
            service = configuration["services"][name]
            self.assertFalse(any(volume["type"] == "bind" for volume in service.get("volumes", [])))
            self.assertNotIn("watch", " ".join(service["command"]))
        self.assertEqual(configuration["services"]["tool"]["build"]["target"], "test")

    def test_dev_dependencies_are_image_owned_and_runtimes_use_installed_binary(self):
        configuration = self.configuration("dev")
        for name in ("tool", "dev-app", "dev-scheduler", "dev-frontend", "dev-builder"):
            for volume in configuration["services"][name].get("volumes", []):
                self.assertNotEqual(volume["target"], "/app")
                self.assertNotEqual(volume["target"], "/app/node_modules")
        for name in ("dev-app", "dev-scheduler"):
            command = " ".join(configuration["services"][name]["command"])
            self.assertIn("/app/run/bin/med-tracker", command)
            self.assertNotIn("cargo run", command)
            self.assertNotIn("target/debug/med-tracker", command)
        builder = " ".join(configuration["services"]["dev-builder"]["command"])
        self.assertNotIn("assets/static/css", builder)
        self.assertNotIn("assets/views", builder)
        self.assertIn("assets/static/js/profile-push-worker.js", builder)

    def test_npm_install_only_occurs_in_shared_image_dependency_stage(self):
        dockerfile = (ROOT / "Dockerfile").read_text()
        self.assertIn("AS frontend-dependencies", dockerfile)
        self.assertIn("target=/root/.npm", dockerfile)
        self.assertEqual(dockerfile.count("npm ci"), 1)
        self.assertNotIn("npm-cache:", (ROOT / "compose.local.yaml").read_text())

    def configuration(self, environment, root=ROOT):
        env = {**os.environ, **workflow.cache_names("arm64", root), "MT_TOOL_TARGET": "test" if environment == "test" else "development", "MT_PROJECT": workflow.project(environment, root)}
        return json.loads(subprocess.check_output(workflow.compose(environment) + ["config", "--format", "json"], env=env, text=True))

    def test_dev_test_share_external_caches_but_not_data(self):
        dev, test = self.configuration("dev"), self.configuration("test")
        for name in ("target", "cargo-registry", "cargo-git", "browser-cache"):
            self.assertEqual(dev["volumes"][name]["name"], test["volumes"][name]["name"])
            self.assertTrue(dev["volumes"][name]["external"])
        self.assertNotEqual(dev["volumes"]["database"]["name"], test["volumes"]["database"]["name"])
        self.assertFalse(dev["volumes"]["database"].get("external", False))

    def test_other_checkout_shares_downloads_but_has_its_own_executable(self):
        first = workflow.cache_names("arm64")
        second = workflow.cache_names("arm64", pathlib.Path("/tmp/other-worktree"))
        self.assertEqual(first["MT_CARGO_REGISTRY_CACHE"], second["MT_CARGO_REGISTRY_CACHE"])
        self.assertEqual(first["MT_CARGO_GIT_CACHE"], second["MT_CARGO_GIT_CACHE"])
        self.assertNotEqual(first["MT_TARGET_CACHE"], second["MT_TARGET_CACHE"])

    def test_target_and_browser_cache_are_platform_scoped(self):
        first = workflow.cache_names("arm64")
        second = workflow.cache_names("amd64")
        self.assertNotEqual(first["MT_TARGET_CACHE"], second["MT_TARGET_CACHE"])
        self.assertNotEqual(first["MT_BROWSER_CACHE"], second["MT_BROWSER_CACHE"])

    def test_test_invocation_builds_snapshot_before_running(self):
        with patch.object(workflow, "build_tools") as build, patch.object(workflow, "tool") as tool:
            workflow.main(["test", "unit", "--filter", "domain"])
            build.assert_called_once_with("test")
            self.assertIn("nextest", tool.call_args.args)

    def test_container_and_host_share_database_tasks_without_host_helpers(self):
        root = (ROOT / "Taskfile.yml").read_text()
        container = (ROOT / "Taskfiles/container.yml").read_text()
        self.assertIn("taskfile: Taskfiles/database.yml", root)
        self.assertIn("taskfile: Taskfiles/database.yml", container)
        self.assertNotIn("rails/Taskfile", container)
        self.assertIn("COPY Taskfiles/container.yml Taskfile.yml", (ROOT / "Dockerfile").read_text())

    def test_source_snapshot_build_script_ignores_generated_frontend_outputs(self):
        with tempfile.TemporaryDirectory() as directory:
            executable = pathlib.Path(directory) / "build-script"
            subprocess.run(["rustc", str(ROOT / "build.rs"), "-o", str(executable)], check=True)
            output = subprocess.check_output([str(executable)], cwd=directory, text=True)
            self.assertIn("cargo:rerun-if-changed=build.rs", output)
            self.assertIn("MEDTRACKER_BUILD_COMMIT=unknown", output)
            native = subprocess.check_output([str(executable)], cwd=ROOT, text=True)
            self.assertNotIn("cargo:rerun-if-changed", native)
            self.assertNotIn("MEDTRACKER_BUILD_COMMIT=unknown", native)

    def test_portless_missing_preflight_fails_before_build(self):
        with patch.dict(os.environ, {"PORTLESS": "1"}), patch.object(workflow, "portless_preflight", side_effect=RuntimeError("Install Portless")), patch.object(workflow, "build_tools") as build:
            with self.assertRaisesRegex(RuntimeError, "Install Portless"):
                workflow.main(["dev", "up"])
            build.assert_not_called()

    def test_portless_bypass_and_environment_origin(self):
        with patch.dict(os.environ, {"PORTLESS": "0"}), patch.object(workflow.subprocess, "run") as run:
            self.assertIsNone(workflow.portless_preflight("dev"))
            run.assert_not_called()
        with patch.dict(os.environ, {"PORTLESS": "1", "PORTLESS_PORT": "443"}):
            self.assertNotEqual(workflow.public_origin("dev"), workflow.public_origin("test"))
            self.assertTrue(workflow.public_origin("dev").endswith(".localhost"))

    def test_portless_alias_conflict_is_not_overwritten(self):
        with patch.dict(os.environ, {"PORTLESS": "1"}), patch.object(workflow, "portless") as portless, patch.object(workflow, "localhost_proxy"), patch.object(workflow, "alias_port", return_value=1234):
            with self.assertRaisesRegex(RuntimeError, "owned"):
                workflow.register_alias("dev", "127.0.0.1:5678", 3456)
            portless.assert_not_called()

    def test_portless_list_and_cleanup_only_match_owned_binding(self):
        origin = workflow.public_origin("dev")
        with patch.object(workflow, "localhost_proxy"), patch.object(workflow, "portless", return_value=origin + " -> localhost:5678 (alias)") as cli, patch.object(workflow, "app_binding", return_value="127.0.0.1:5678"):
            self.assertEqual(workflow.alias_port("dev"), 5678)
            workflow.remove_alias("dev")
            self.assertEqual(cli.call_args.args, ("alias", "--remove", workflow.project("dev")))
        with patch.object(workflow, "localhost_proxy"), patch.object(workflow, "alias_port", return_value=5678), patch.object(workflow, "app_binding", return_value=None), patch.object(workflow, "portless") as cli:
            with self.assertRaisesRegex(RuntimeError, "untouched"):
                workflow.remove_alias("dev")
            cli.assert_not_called()

    def test_portless_alias_operations_reject_multiple_tlds(self):
        with patch.object(workflow, "portless", return_value="Mode: HTTPS, .localhost, .test"):
            with self.assertRaisesRegex(RuntimeError, "exclusively"):
                workflow.localhost_proxy()

    def test_bypass_reuses_owned_port_and_sets_matching_origin(self):
        with patch.object(workflow, "app_binding", return_value="127.0.0.1:5678"), patch.object(workflow.socket, "socket") as socket:
            self.assertEqual(workflow.bypass_endpoint("dev"), ("http://127.0.0.1:5678", 5678))
            socket.assert_not_called()
        with patch.object(workflow, "app_binding", return_value=None), patch.object(workflow.socket, "socket") as socket:
            socket.return_value.__enter__.return_value.getsockname.return_value = ("127.0.0.1", 6789)
            self.assertEqual(workflow.bypass_endpoint("test"), ("http://127.0.0.1:6789", 6789))

    def test_registered_alias_health_allows_bounded_proxy_route_propagation(self):
        with patch.dict(os.environ, {"PORTLESS": "1"}), patch.object(workflow, "localhost_proxy"), patch.object(workflow, "alias_port", return_value=None), patch.object(workflow, "portless"), patch.object(workflow.subprocess, "run") as curl:
            workflow.register_alias("dev", "127.0.0.1:5678", None)
            command = curl.call_args.args[0]
            self.assertIn("--retry-all-errors", command)
            self.assertIn("--retry-max-time", command)
            self.assertNotIn("--insecure", command)

    def test_driver_exists(self):
        self.assertTrue((ROOT / "scripts/compose_workflows.py").exists())

    def test_obsolete_namespaces_removed(self):
        text = (ROOT / "Taskfile.yml").read_text()
        self.assertNotIn("taskfile: rust/", text)

    def test_checkout_and_environment_are_isolated(self):
        self.assertNotEqual(workflow.project("dev"), workflow.project("test"))
        self.assertNotEqual(workflow.project("dev"), workflow.project("dev", pathlib.Path("/tmp/another-checkout")))

    def test_production_mutations_are_rejected_before_docker(self):
        for action in ("seed", "reset"):
            with self.subTest(action=action), patch.object(workflow, "run") as run:
                with self.assertRaises(SystemExit):
                    workflow.main(["prod", action])
                run.assert_not_called()

    def test_reset_requires_exact_project_and_preserves_caches(self):
        with patch.object(workflow, "remove_alias"), patch.object(workflow, "check_data_volumes"), patch.object(workflow, "run") as run, patch.object(workflow.subprocess, "run") as docker:
            with self.assertRaises(SystemExit):
                workflow.main(["dev", "reset"])
            run.assert_not_called()
            workflow.main(["dev", "reset", "--confirm", workflow.project("dev")])
            self.assertNotIn("--volumes", str(run.call_args_list))
            self.assertNotIn("cargo-registry", str(docker.call_args_list))
            self.assertIn("scanner-data", str(docker.call_args_list))

    def test_filter_is_one_argument_and_target_is_explicit(self):
        with patch.object(workflow, "build_tools"), patch.object(workflow, "services"), patch.object(workflow, "tool") as tool:
            workflow.main(["test", "integration", "--target", "care_api", "--filter", "fixture; touch /tmp/unwanted"])
            command = tool.call_args_list[-1].args
            self.assertEqual(command[-1], "fixture; touch /tmp/unwanted")
            self.assertIn("care_api", command)
            self.assertIn("--no-tests=fail", command)

    def test_tool_failure_propagates(self):
        with patch.object(workflow, "build_tools"), patch.object(workflow, "services"), patch.object(workflow, "tool", side_effect=subprocess.CalledProcessError(7, "docker")):
            with self.assertRaises(subprocess.CalledProcessError):
                workflow.main(["test", "unit"])

    def test_unfiltered_integration_keeps_process_concurrency_proof(self):
        with patch.object(workflow, "build_tools"), patch.object(workflow, "services"), patch.object(workflow, "tool") as tool:
            workflow.main(["test", "integration"])
            self.assertEqual(tool.call_args_list[-1].args[-4:], ("competing_processes_prepare_once", "--", "--ignored", "--exact"))

    def test_unit_target_selects_workspace_package(self):
        with patch.object(workflow, "build_tools"), patch.object(workflow, "services"), patch.object(workflow, "tool") as tool:
            workflow.main(["test", "unit", "--target", "med-tracker", "--filter", "domain"])
            self.assertIn("--package", tool.call_args_list[-1].args)
            self.assertIn("med-tracker", tool.call_args_list[-1].args)

    def test_unit_does_not_provision_database_or_storage(self):
        with patch.object(workflow, "build_tools"), patch.object(workflow, "services") as services, patch.object(workflow, "tool"):
            workflow.main(["test", "unit", "--filter", "domain"])
            services.assert_not_called()

    def test_frontend_watch_is_separate_from_backend(self):
        services = self.configuration("dev")["services"]
        self.assertIn("--watch=always", services["dev-frontend"]["command"])
        command = services["dev-frontend"]["command"]
        self.assertEqual(command[command.index("--cwd") + 1], "/app/assets")
        self.assertNotIn("assets/static/css", " ".join(services["dev-app"]["command"]))
        self.assertIn("assets/views", services["dev-app"]["command"])

    def test_production_config_has_one_tmpfs_and_no_source_mount(self):
        env = {**os.environ, **workflow.cache_names("arm64"), "MT_TOOL_TARGET": "development", "MT_PROJECT": "mt-tooling-config"}
        configuration = subprocess.check_output(workflow.compose("prod") + ["config", "--format", "json"], env=env, text=True)
        app = json.loads(configuration)["services"]["app"]
        self.assertEqual(app["tmpfs"], ["/tmp:uid=65532,gid=65532"])
        self.assertFalse(app.get("volumes"))
        self.assertNotEqual(app["environment"].get("MEDTRACKER_COOKIE_SECURE"), "false")

    def test_host_browser_target_is_forwarded_in_direct_and_owned_paths(self):
        for owned in ("1", ""):
            env = {**os.environ, "CI": "true", "MEDTRACKER_OWNED_DATABASE": owned}
            output = subprocess.check_output(["task", "--dry", "--verbose", "test:browser", "TARGET=missing-binary", "FILTER=login"], cwd=ROOT, env=env, stderr=subprocess.STDOUT, text=True)
            self.assertNotIn("--test browser", output)
            self.assertIn("missing-binary", output)

    def test_up_starts_test_application(self):
        with patch.object(workflow, "portless_preflight"), patch.object(workflow, "register_alias"), patch.object(workflow, "wait_ready") as ready, patch.object(workflow, "services"), patch.object(workflow, "tool"), patch.object(workflow, "application"), patch.object(workflow, "run") as run:
            run.return_value.stdout = "t"
            workflow.main(["test", "up"])
            self.assertIn("test-app", str(run.call_args_list))
            self.assertNotIn("ALTER ROLE", str(run.call_args_list))
            ready.assert_called_once_with("test")

    def test_existing_privileged_runtime_identity_is_rejected(self):
        with patch.object(workflow, "portless_preflight"), patch.object(workflow, "register_alias"), patch.object(workflow, "wait_ready"), patch.object(workflow, "services"), patch.object(workflow, "tool"), patch.object(workflow, "application"), patch.object(workflow, "run") as run:
            run.return_value.stdout = "f"
            with self.assertRaisesRegex(RuntimeError, "restricted runtime"):
                workflow.main(["dev", "up"])

    def test_readiness_retries_connection_closed_during_boot(self):
        with patch.object(workflow, "run") as run, patch.object(workflow.time, "sleep"), patch.object(workflow.urllib.request, "urlopen") as urlopen:
            run.return_value.stdout = "127.0.0.1:5150"
            response = unittest.mock.MagicMock()
            response.__enter__.return_value.status = 200
            urlopen.side_effect = [http.client.RemoteDisconnected("starting"), response]
            workflow.wait_ready("dev")
            self.assertEqual(urlopen.call_count, 2)

    def test_unowned_volume_is_never_adopted(self):
        result = subprocess.CompletedProcess([], 0, stdout='[{"Labels": {}}]', stderr="")
        with patch.object(workflow.subprocess, "run", return_value=result):
            with self.assertRaisesRegex(RuntimeError, "unowned volume"):
                workflow.check_data_volumes("dev")

    def test_missing_volume_is_allowed_but_daemon_failure_is_not(self):
        for message in ("No such volume", "Error response from daemon: get volume: no such volume"):
            missing = subprocess.CompletedProcess([], 1, stdout="", stderr=message)
            with patch.object(workflow.subprocess, "run", return_value=missing):
                workflow.check_data_volumes("dev")
        failed = subprocess.CompletedProcess([], 1, stdout="", stderr="permission denied")
        with patch.object(workflow.subprocess, "run", return_value=failed):
            with self.assertRaisesRegex(RuntimeError, "permission denied"):
                workflow.check_data_volumes("dev")

if __name__ == "__main__":
    unittest.main()
