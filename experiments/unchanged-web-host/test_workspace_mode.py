"""Protect live credentials against accidentally selecting fixture mode."""
import json,tempfile,unittest
from pathlib import Path
from run_probe import claim_workspace_mode

class WorkspaceModeTests(unittest.TestCase):
    def test_live_workspace_cannot_be_reopened_as_fixture(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);claim_workspace_mode(root,'live')
            config=root/'external-ui-state.json';config.write_text(json.dumps({'llmConfig':{'provider':'custom','model':'real','apiKey':'synthetic-test-secret'}}))
            original=config.read_bytes()
            with self.assertRaises(ValueError):claim_workspace_mode(root,'fixture')
            self.assertEqual(config.read_bytes(),original)
            self.assertEqual((root/'.external-host-model-mode').stat().st_mode&0o777,0o600)

    def test_legacy_live_config_is_not_reset_without_a_marker(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);config=root/'external-ui-state.json'
            config.write_text(json.dumps({'llmConfig':{'provider':'ollama','model':'local-model','apiKey':''}}));original=config.read_bytes()
            with self.assertRaises(ValueError):claim_workspace_mode(root,'fixture')
            self.assertEqual(config.read_bytes(),original)
            claim_workspace_mode(root,'live');claim_workspace_mode(root,'live')

    def test_legacy_fixture_and_fixture_marker_are_separate_from_live(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);(root/'external-ui-state.json').write_text(json.dumps({'llmConfig':{'provider':'custom','model':'fixture','apiKey':''}}))
            with self.assertRaises(ValueError):claim_workspace_mode(root,'live')
            claim_workspace_mode(root,'fixture')
            with self.assertRaises(ValueError):claim_workspace_mode(root,'live')

    def test_symlink_marker_is_rejected(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);target=root/'target';target.write_text('live\n')
            (root/'.external-host-model-mode').symlink_to(target)
            with self.assertRaises(ValueError):claim_workspace_mode(root,'live')

if __name__=='__main__':unittest.main()
