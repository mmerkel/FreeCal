# Tests of switched_off_tests.py on small fixture trees.
# Run: python3 -m unittest discover -s .husky -p 'test_*.py'
import os
import subprocess
import sys
import tempfile
import unittest

SCRIPT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "switched_off_tests.py")


class SwitchedOffTests(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.root = self.dir.name
        self.ticket("180", "ready-for-agent")
        self.ticket("173", "done")

    def tearDown(self):
        self.dir.cleanup()

    def ticket(self, num, status):
        self.write(f".scratch/freecal-v1/issues/{num}-a-ticket.md", f"# {num}: A ticket\n\n**Status:** {status}\n")

    def write(self, path, text):
        full = os.path.join(self.root, path)
        os.makedirs(os.path.dirname(full), exist_ok=True)
        with open(full, "w") as f:
            f.write(text)

    def run_check(self):
        result = subprocess.run([sys.executable, SCRIPT, self.root], capture_output=True, text=True)
        return result.returncode, result.stdout + result.stderr

    def assert_passes(self):
        code, out = self.run_check()
        self.assertEqual(code, 0, out)

    def assert_refused(self, location, reason):
        code, out = self.run_check()
        self.assertEqual(code, 1, out)
        self.assertIn(location, out)
        self.assertIn(reason, out)

    def test_an_empty_tree_passes(self):
        self.assert_passes()

    def test_a_rust_note_naming_an_open_ticket_passes(self):
        self.write("core/tests/undo.rs", '#[test]\n#[ignore = "until ticket 180: Undo reads the Event Versions"]\nfn undoes() {}\n')
        self.assert_passes()

    def test_a_frontend_note_naming_an_open_ticket_passes(self):
        for i, kind in enumerate(["it", "describe", "test"]):
            self.write(
                f"src/lib/a{i}.test.ts",
                f'import {{ {kind} }} from "vitest";\n  // until ticket 180: the Undo report replaces the refusal\n  {kind}.skip("undoes", () => {{}});\n',
            )
        self.assert_passes()

    def test_a_by_hand_note_is_refused(self):
        self.write("src-tauri/src/keyring.rs", '#[test]\n#[ignore = "by hand: needs a desktop session\'s keyring"]\nfn stores() {}\n')
        self.assert_refused("src-tauri/src/keyring.rs:2", "until ticket NN")

    def test_a_bare_rust_ignore_is_refused(self):
        self.write("core/src/a.rs", "\n#[test]\n#[ignore]\nfn a() {}\n")
        self.assert_refused("core/src/a.rs:3", "until ticket NN")

    def test_a_rust_note_of_another_form_is_refused(self):
        self.write("core/src/a.rs", '#[ignore = "flaky"]\nfn a() {}\n')
        self.assert_refused("core/src/a.rs:1", "until ticket NN")

    def test_a_rust_note_without_a_reason_is_refused(self):
        self.write("core/src/a.rs", '#[ignore = "until ticket 180"]\nfn a() {}\n')
        self.assert_refused("core/src/a.rs:1", "until ticket NN")

    def test_a_bare_frontend_skip_is_refused(self):
        self.write("src/a.test.ts", 'it("a", () => {});\ndescribe.skip("b", () => {});\n')
        self.assert_refused("src/a.test.ts:2", "until ticket NN")

    def test_a_frontend_note_in_the_name_is_refused(self):
        self.write("src/a.test.ts", 'it.skip("until ticket 180: x", () => {});\n')
        self.assert_refused("src/a.test.ts:1", "until ticket NN")

    def test_a_frontend_note_two_lines_up_is_refused(self):
        self.write("src/a.test.ts", "// until ticket 180: x\n\ntest.skip(\"a\", () => {});\n")
        self.assert_refused("src/a.test.ts:3", "until ticket NN")

    def test_a_frontend_note_left_after_the_skip_is_gone_is_refused(self):
        self.write("src/a.test.ts", "// until ticket 180: x\nit(\"a\", () => {});\n")
        self.assert_refused("src/a.test.ts:1", "needs a skipped test on the next line")

    def test_a_ticket_with_no_file_is_refused(self):
        self.write("core/src/a.rs", '#[ignore = "until ticket 999: x"]\nfn a() {}\n')
        self.assert_refused("core/src/a.rs:1", "no ticket 999")

    def test_a_done_ticket_is_refused(self):
        self.write("src/a.test.ts", "// until ticket 173: x\nit.skip(\"a\", () => {});\n")
        self.assert_refused("src/a.test.ts:2", "ticket 173 is done")

    def test_a_wontfix_ticket_is_refused(self):
        self.ticket("150", "wontfix")
        self.write("core/src/a.rs", '#[ignore = "until ticket 150: x"]\nfn a() {}\n')
        self.assert_refused("core/src/a.rs:1", "ticket 150 is wontfix")

    def test_every_refusal_is_listed(self):
        self.write("core/src/a.rs", "#[ignore]\nfn a() {}\n")
        self.write("src/b.test.ts", "it.skip(\"b\", () => {});\n")
        code, out = self.run_check()
        self.assertEqual(code, 1, out)
        self.assertIn("core/src/a.rs:1", out)
        self.assertIn("src/b.test.ts:1", out)

    def test_other_directories_and_build_output_are_not_searched(self):
        self.write("docs/a.rs", "#[ignore]\n")
        self.write("src-tauri/target/debug/build/a.rs", "#[ignore]\n")
        self.write("node_modules/x/a.test.ts", "it.skip(\"a\", () => {});\n")
        self.assert_passes()


if __name__ == "__main__":
    unittest.main()
