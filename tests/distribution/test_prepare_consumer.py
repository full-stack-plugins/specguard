import importlib.util
import pathlib
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('prepare', ROOT / 'scripts/prepare-source-consumer.py')
prepare = importlib.util.module_from_spec(spec)
spec.loader.exec_module(prepare)

class DestinationTests(unittest.TestCase):
    def test_dangling_destination_symlink_is_rejected_before_archive_read(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            redirected = root / 'redirected'
            link = root / 'destination'
            link.symlink_to(redirected, target_is_directory=True)
            with self.assertRaises(ValueError):
                prepare.prepare(root / 'nonexistent-archive.tar', link)
            self.assertFalse(redirected.exists())
            self.assertTrue(link.is_symlink())

if __name__ == '__main__':
    unittest.main()
