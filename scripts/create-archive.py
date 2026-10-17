"""Create a new ZIP, preserving bytes and clamping only ZIP's limited timestamp metadata."""
import argparse
from pathlib import Path
import zipfile


def archive(directory, output, include_root=False):
    directory = directory.resolve(strict=True)
    output = output.resolve()
    if not directory.is_dir() or output.is_relative_to(directory):
        raise ValueError("Archive output must be outside its source directory")
    base = directory.parent if include_root else directory
    with zipfile.ZipFile(output, "x", zipfile.ZIP_DEFLATED, strict_timestamps=False) as target:
        for path in sorted(directory.rglob("*")):
            relative = path.relative_to(directory)
            if "__pycache__" in relative.parts or path.suffix in (".pyc", ".pyo"):
                continue
            if path.is_symlink():
                raise ValueError("Stage real files before creating the archive")
            if path.is_file():
                target.write(path, path.relative_to(base).as_posix())


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--include-root", action="store_true")
    args = parser.parse_args()
    archive(args.directory, args.output, args.include_root)
