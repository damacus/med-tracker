import importlib.util
import pathlib
import unittest
from unittest.mock import patch
import subprocess
import http.client
import json
import os

ROOT = pathlib.Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("workflows", ROOT / "scripts/compose_workflows.py")
workflow = importlib.util.module_from_spec(spec)
spec.loader.exec_module(workflow)

class Workflows(unittest.TestCase):
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
        with patch.object(workflow, "check_data_volumes"), patch.object(workflow, "run") as run, patch.object(workflow.subprocess, "run") as docker:
            with self.assertRaises(SystemExit):
                workflow.main(["dev", "reset"])
            run.assert_not_called()
            workflow.main(["dev", "reset", "--confirm", workflow.project("dev")])
            self.assertNotIn("--volumes", str(run.call_args_list))
            self.assertNotIn("cargo-registry", str(docker.call_args_list))
            self.assertIn("scanner-data", str(docker.call_args_list))

    def test_filter_is_one_argument_and_target_is_explicit(self):
        with patch.object(workflow, "services"), patch.object(workflow, "tool") as tool:
            workflow.main(["test", "integration", "--target", "care_api", "--filter", "fixture; touch /tmp/unwanted"])
            command = tool.call_args_list[-1].args
            self.assertEqual(command[-1], "fixture; touch /tmp/unwanted")
            self.assertIn("care_api", command)
            self.assertIn("--no-tests=fail", command)

    def test_tool_failure_propagates(self):
        with patch.object(workflow, "services"), patch.object(workflow, "tool", side_effect=subprocess.CalledProcessError(7, "docker")):
            with self.assertRaises(subprocess.CalledProcessError):
                workflow.main(["test", "unit"])

    def test_unfiltered_integration_keeps_process_concurrency_proof(self):
        with patch.object(workflow, "services"), patch.object(workflow, "tool") as tool:
            workflow.main(["test", "integration"])
            self.assertEqual(tool.call_args_list[-1].args[-4:], ("competing_processes_prepare_once", "--", "--ignored", "--exact"))

    def test_unit_target_selects_workspace_package(self):
        with patch.object(workflow, "services"), patch.object(workflow, "tool") as tool:
            workflow.main(["test", "unit", "--target", "med-tracker", "--filter", "domain"])
            self.assertIn("--package", tool.call_args_list[-1].args)
            self.assertIn("med-tracker", tool.call_args_list[-1].args)

    def test_unit_does_not_provision_database_or_storage(self):
        with patch.object(workflow, "services") as services, patch.object(workflow, "tool"):
            workflow.main(["test", "unit", "--filter", "domain"])
            services.assert_not_called()

    def test_asset_reload_prepares_css_before_restart(self):
        text = (ROOT / "compose.local.yaml").read_text()
        self.assertIn("task --taskfile Taskfiles/frontend.yml --dir /app prepare && cargo run", text)
        self.assertIn("assets/static/css/compiled.css", text)

    def test_production_config_has_one_tmpfs_and_no_source_mount(self):
        env = {**os.environ, "MT_PROJECT": "mt-tooling-config"}
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
        with patch.object(workflow, "wait_ready") as ready, patch.object(workflow, "services"), patch.object(workflow, "tool"), patch.object(workflow, "application"), patch.object(workflow, "run") as run:
            run.return_value.stdout = "t"
            workflow.main(["test", "up"])
            self.assertIn("test-app", str(run.call_args_list))
            self.assertNotIn("ALTER ROLE", str(run.call_args_list))
            ready.assert_called_once_with("test")

    def test_existing_privileged_runtime_identity_is_rejected(self):
        with patch.object(workflow, "wait_ready"), patch.object(workflow, "services"), patch.object(workflow, "tool"), patch.object(workflow, "application"), patch.object(workflow, "run") as run:
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
