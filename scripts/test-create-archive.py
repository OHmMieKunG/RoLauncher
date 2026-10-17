"""ZIP fixtures; no installer, application or account activity."""
import importlib.util
from pathlib import Path
import os
import tempfile
import unittest
import zipfile

spec = importlib.util.spec_from_file_location("archive", Path(__file__).with_name("create-archive.py"))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ArchiveTests(unittest.TestCase):
    def test_old_timestamp_clamped_without_changing_source_bytes_or_time(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / "payload with spaces"
            source.mkdir()
            license = source / "LICENSE"
            license.write_bytes(b"unchanged license\n")
            os.utime(license, (1, 1))
            before = license.stat().st_mtime_ns
            output = root / "old.zip"
            module.archive(source, output, include_root=True)
            with zipfile.ZipFile(output) as archive:
                name = source.name + "/LICENSE"
                self.assertEqual(archive.read(name), license.read_bytes())
                self.assertEqual(archive.getinfo(name).date_time[:3], (1980, 1, 1))
            self.assertEqual(license.stat().st_mtime_ns, before)

    def test_source_layout_hidden_files_and_nested_cache_exclusion(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / "source"
            (source / "scripts/__pycache__").mkdir(parents=True)
            (source / ".gitignore").write_text("fixture")
            (source / "scripts/helper.py").write_text("source")
            (source / "scripts/__pycache__/helper.pyc").write_bytes(b"cache")
            (source / "scripts/old.pyo").write_bytes(b"cache")
            output = root / "source.zip"
            module.archive(source, output)
            with zipfile.ZipFile(output) as archive:
                self.assertEqual(set(archive.namelist()), {".gitignore", "scripts/helper.py"})
                self.assertIsNone(archive.testzip())
            with self.assertRaises(FileExistsError):
                module.archive(source, output)

    def test_rejects_output_inside_source(self):
        with tempfile.TemporaryDirectory() as temp:
            source = Path(temp)
            with self.assertRaises(ValueError):
                module.archive(source, source / "recursive.zip")
            self.assertFalse((source / "recursive.zip").exists())


if __name__ == "__main__":
    unittest.main()
