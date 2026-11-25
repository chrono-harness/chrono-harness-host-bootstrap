"""Exercise the registered installer with external tools replaced at its process boundary."""
import os
from pathlib import Path
import subprocess
import sys
import shutil
import tempfile
import unittest


INSTALL = Path(__file__).with_name("install.sh").resolve()


def mock(name, args):
    env = os.environ
    with open(env["CALLS"], "a") as log:
        print(name, *args, file=log)
        if name == "shasum":
            print("checksum", sys.stdin.read().rstrip(), file=log)
    if name == "uname":
        print(env["SYSTEM" if args[0] == "-s" else "MACHINE"])
    elif name == "curl":
        shutil.copyfile(env["MANAGER"], args[-1])
    elif name == "shasum":
        return int(env["DIGEST_EXIT"])
    elif name == "mktemp":
        print(tempfile.mkdtemp(dir=Path(args[-1]).parent, prefix="bootstrap-mise."))
    elif name == "rm":
        shutil.rmtree(args[-1])
    elif name == "chmod":
        Path(args[-1]).chmod(int(args[0], 8))
    elif name == "mkdir":
        Path(args[-1]).mkdir(parents=True, exist_ok=True)
    elif name == "ln":
        p = Path(args[-1])
        if p.is_symlink():
            p.unlink()
        p.symlink_to(args[-2])
    elif name == "dirname":
        print(Path(args[-1]).parent)
    elif name == "mise":
        store = Path(env["STORE"])
        if args[0] == "which":
            path = store / args[1]
            if not path.is_file():
                return 1
            print(path)
        elif args[0] == "install":
            (store / args[1].removeprefix("http:")).touch()
        else:
            raise AssertionError(args)
    else:
        raise AssertionError(name)
    return 0


class FirstInstall(unittest.TestCase):
    def check_case(self, *, installed=False, hosted=False, system="Linux", machine="x86_64", digest_ok=True):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            commands = root / "commands"
            commands.mkdir()
            log = root / "calls"
            log.touch()
            store = root / "tools"
            store.mkdir()
            manager = root / "manager"
            wrapper = "#!" + sys.executable + "\n" + (
                "import runpy,sys\nfrom pathlib import Path\n"
                f"raise SystemExit(runpy.run_path({str(Path(__file__).resolve())!r})['mock']("
                "Path(sys.argv[0]).name,sys.argv[1:]))\n"
            )
            manager.write_text(wrapper)
            manager.chmod(0o755)
            if installed:
                (commands / "mise").symlink_to(manager)
            for name in ("uname", "curl", "shasum", "mktemp", "mkdir", "rm", "chmod", "ln", "dirname"):
                (commands / name).symlink_to(manager)
            env = dict(PATH=str(commands), STORE=str(store), CALLS=str(log), MANAGER=str(manager),
                       GITHUB_ACTIONS="true" if hosted else "", RUNNER_TEMP=str(root),
                       SYSTEM=system, MACHINE=machine, DIGEST_EXIT="0" if digest_ok else "1")
            result = subprocess.run(["/bin/sh", INSTALL], cwd=root, env=env, capture_output=True, text=True)
            links = root / ".chrono-harness/bin"
            if result.returncode == 0:
                expected = INSTALL.read_text().split("set -- ",1)[1].splitlines()[0].split()
                self.assertEqual(sorted(p.name for p in links.iterdir()), sorted(expected))
                self.assertTrue(all(p.resolve().parent == store for p in links.iterdir()))
                first = (links / expected[0]).resolve()
                again = subprocess.run(["/bin/sh", INSTALL], cwd=root, env=env, capture_output=True, text=True)
                self.assertEqual(again.returncode, 0, again.stderr)
                self.assertEqual((links / expected[0]).resolve(), first)
            else:
                self.assertFalse(links.exists())
            self.assertFalse(list(root.glob("bootstrap-mise.*")))
            return result, log.read_text()

    def test_available_manager_and_missing_local_manager(self):
        result, calls = self.check_case(installed=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotIn("curl ", calls)
        result, calls = self.check_case()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(calls, "")

    def test_two_registered_platforms_and_digest_failure(self):
        for system, machine, asset, digest in [
            ("Darwin","arm64","macos-arm64","87b2bd5a1b88a612ee8856df23499404b115870b5a8a8e008763c7cd51c8d412"),
            ("Linux","x86_64","linux-x64","9c12c24f5eac6b6357e5265e2629166cc3005459037f5d747680a99e375ff2ab"),
        ]:
            result, calls = self.check_case(hosted=True, system=system, machine=machine)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("mise-v2026.11.12-" + asset, calls)
            self.assertIn("checksum " + digest + "  mise", calls)
        result, calls = self.check_case(hosted=True, digest_ok=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn("mise which", calls)
        result, calls = self.check_case(hosted=True, machine="unsupported")
        self.assertIn("Unsupported runner platform", result.stderr)
        self.assertNotIn("curl ", calls)


if __name__ == "__main__":
    unittest.main()
