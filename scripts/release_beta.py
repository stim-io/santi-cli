#!/usr/bin/env python3

from __future__ import annotations

import argparse
import hashlib
import shutil
import subprocess
import tarfile
import tempfile
import zipfile
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parent.parent
DIST_DIR = REPO_ROOT / "dist"
APP_CARGO_TOML = REPO_ROOT / "app" / "Cargo.toml"
CRATE_NAME = "santi-cli"
BINARY_NAME = "santi-cli"


def main() -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)

    preflight_parser = subparsers.add_parser("preflight")
    preflight_parser.add_argument("--version", required=True)

    package_parser = subparsers.add_parser("package")
    package_parser.add_argument("--version", required=True)
    package_parser.add_argument("--target", required=True)

    collate_parser = subparsers.add_parser("collate-checksums")
    collate_parser.add_argument("--dist-dir", default=str(DIST_DIR))

    args = parser.parse_args()

    if args.command == "preflight":
        preflight(args.version)
        return 0

    if args.command == "package":
        package(args.version, args.target)
        return 0

    if args.command == "collate-checksums":
        collate_checksums(Path(args.dist_dir))
        return 0

    raise SystemExit(f"unsupported command: {args.command}")


def preflight(version: str) -> None:
    parse_beta_version(version)

    crate_version = read_crate_version()
    if crate_version != version:
        raise SystemExit(
            f"release version mismatch: got {version}, expected {crate_version} from app/Cargo.toml",
        )

    tag = build_tag(version)
    existing = capture(["git", "tag", "--list", tag]).strip()
    if existing == tag:
        raise SystemExit(f"release tag {tag} already exists")


def package(version: str, target: str) -> None:
    preflight(version)

    tag = build_tag(version)
    archive_stem = f"{CRATE_NAME}-{tag}-{target}"
    DIST_DIR.mkdir(parents=True, exist_ok=True)

    run(["cargo", "build", "--release", "--locked", "--target", target, "-p", CRATE_NAME])

    with tempfile.TemporaryDirectory() as temp_dir:
        stage_dir = Path(temp_dir)
        binary_path = release_binary_path(target)
        staged_binary = stage_dir / binary_path.name
        shutil.copy2(binary_path, staged_binary)

        if "windows" in target:
            archive_path = DIST_DIR / f"{archive_stem}.zip"
            with zipfile.ZipFile(archive_path, "w", compression=zipfile.ZIP_DEFLATED) as archive:
                archive.write(staged_binary, arcname=staged_binary.name)
        else:
            archive_path = DIST_DIR / f"{archive_stem}.tar.gz"
            with tarfile.open(archive_path, "w:gz") as archive:
                archive.add(staged_binary, arcname=staged_binary.name)

    write_checksum_file(DIST_DIR / f"checksums-{target}.txt", [archive_path])


def collate_checksums(dist_dir: Path) -> None:
    checksum_files = sorted(dist_dir.rglob("checksums-*.txt"))
    if not checksum_files:
        raise SystemExit(f"no checksum files found under {dist_dir}")

    lines: set[str] = set()
    for file_path in checksum_files:
        for line in file_path.read_text(encoding="utf-8").splitlines():
            stripped = line.strip()
            if stripped:
                lines.add(stripped)

    output_path = dist_dir / "checksums.txt"
    output_path.write_text("\n".join(sorted(lines)) + "\n", encoding="utf-8")


def release_binary_path(target: str) -> Path:
    binary_name = f"{BINARY_NAME}.exe" if "windows" in target else BINARY_NAME
    return REPO_ROOT / "target" / target / "release" / binary_name


def write_checksum_file(output_path: Path, artifacts: list[Path]) -> None:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    lines = []
    for artifact in artifacts:
        digest = hashlib.sha256(artifact.read_bytes()).hexdigest()
        lines.append(f"{digest}  {artifact.name}")
    output_path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def read_crate_version() -> str:
    in_package = False
    for raw_line in APP_CARGO_TOML.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if line == "[package]":
            in_package = True
            continue
        if in_package and line.startswith("["):
            break
        if in_package and line.startswith("version"):
            _, value = line.split("=", 1)
            return value.strip().strip('"')

    raise SystemExit(f"failed to read package version from {APP_CARGO_TOML}")


def build_tag(version: str) -> str:
    return f"v{version}"


def parse_beta_version(version: str) -> None:
    if not version.startswith("0.1.0-beta."):
        raise SystemExit(
            f"only long-lived beta versions 0.1.0-beta.N are allowed, got {version}",
        )

    try:
        int(version.removeprefix("0.1.0-beta."))
    except ValueError as error:
        raise SystemExit(
            f"only long-lived beta versions 0.1.0-beta.N are allowed, got {version}",
        ) from error


def run(args: list[str]) -> None:
    print(f"+ {' '.join(args)}")
    subprocess.run(args, cwd=REPO_ROOT, check=True)


def capture(args: list[str]) -> str:
    print(f"+ {' '.join(args)}")
    completed = subprocess.run(
        args,
        cwd=REPO_ROOT,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return completed.stdout


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except subprocess.CalledProcessError as error:
        raise SystemExit(error.returncode) from error
