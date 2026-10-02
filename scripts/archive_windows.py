#!/usr/bin/env python3
"""Create a stable Windows release ZIP and its SHA-256 line."""

from __future__ import annotations

import argparse
import hashlib
from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile, ZipInfo


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--version", required=True)
    parser.add_argument("--arch", choices=("x86_64", "i686"), required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--repository", type=Path, default=Path.cwd())
    parser.add_argument("--output", type=Path, default=Path.cwd())
    args = parser.parse_args()

    archive_name = f"chronogrid-{args.version}-windows-{args.arch}.zip"
    archive_path = args.output / archive_name
    files = [
        ("chronogrid.exe", args.binary),
        ("LICENSE", args.repository / "LICENSE"),
        ("README.md", args.repository / "README.md"),
    ]
    args.output.mkdir(parents=True, exist_ok=True)
    with ZipFile(archive_path, "w", compression=ZIP_DEFLATED, compresslevel=9) as archive:
        for name, path in sorted(files):
            info = ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            info.compress_type = ZIP_DEFLATED
            info.create_system = 3
            info.external_attr = 0o100644 << 16
            archive.writestr(info, path.read_bytes(), compress_type=ZIP_DEFLATED, compresslevel=9)

    digest = hashlib.sha256(archive_path.read_bytes()).hexdigest()
    (args.output / f"sha-{args.arch}.txt").write_text(
        f"{digest}  {archive_name}\n", encoding="ascii"
    )


if __name__ == "__main__":
    main()
