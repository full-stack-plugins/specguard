#!/usr/bin/env python3
"""Create a new, local test consumer from a verified source bundle (offline registry cache required)."""
import argparse, importlib.util, pathlib, shutil
spec = importlib.util.spec_from_file_location('bundle', pathlib.Path(__file__).with_name('source-bundle.py'))
bundle = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bundle)

def prepare(archive, destination):
    # Preserve the requested leaf so unpack_checked can reject an existing
    # (including dangling) symlink. Parent ownership is the caller boundary.
    root = pathlib.Path(destination).absolute()
    bundle.unpack_checked(archive, root)
    consumer = root / 'consumer'
    (consumer / 'src').mkdir(parents=True)
    source = root / 'sources/specguard'
    manifest = (source / 'Cargo.toml').read_text()
    manifest = manifest.replace('name = "specguard"', 'name = "specguard-local-consumer"', 1)
    manifest = manifest.replace('[dependencies]', '[dependencies]\nspecguard = { path = "../sources/specguard" }', 1)
    manifest = manifest.replace('path = "../gitguard"', 'path = "../sources/gitguard"').replace('path = "../guardengine"', 'path = "../sources/guardengine"')
    (consumer / 'Cargo.toml').write_text(manifest)
    shutil.copyfile(source / 'Cargo.lock', consumer / 'Cargo.lock')
    matrix = (source / 'tests/compatibility_matrix.rs').read_text()
    for name in ('common', 'producer_support'):
        matrix = matrix.replace(f'mod {name};', f'#[path = "../../sources/specguard/tests/{name}/mod.rs"]\nmod {name};')
    matrix = matrix.replace('../fixtures/', '../../sources/specguard/fixtures/')
    (consumer / 'src/lib.rs').write_text('#![cfg(test)]\n' + matrix)
    return consumer

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archive'); parser.add_argument('destination')
    args = parser.parse_args()
    print(prepare(args.archive, args.destination))
