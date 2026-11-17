#!/usr/bin/env python3
"""Create a checksummed source snapshot and build an Arch package without installing it."""
import argparse
import gzip
import hashlib
import os
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import tempfile
import tomllib


ROOT = Path(__file__).resolve().parents[2]
FILES = (
    "Cargo.toml", "Cargo.lock", "backend.py", "requirements.txt", "LICENSE", "README.md",
    "assets/Tektur-Medium.ttf", "assets/Tektur-OFL.txt",
    "packaging/arch/PKGBUILD", "packaging/arch/build-local.py",
    "packaging/arch/skylauncher.desktop", "packaging/arch/skylauncher.svg",
    "packaging/arch/python-minecraft-launcher-lib/PKGBUILD",
    "packaging/windows/build-windows.sh", "packaging/windows/installer.nsi",
    "packaging/windows/LEEME-Windows.txt",
)
TREES = (("src", ".rs"), ("tests", ".py"), ("docs", ".md"))


def source_files(root):
    files = {root / name for name in FILES}
    for directory, suffix in TREES:
        for path in (root / directory).rglob("*"):
            if path.is_symlink():
                raise ValueError(f"No se empaquetan enlaces simbólicos: {path}")
            if path.is_file() and path.suffix == suffix and "__pycache__" not in path.parts:
                files.add(path)
    for path in files:
        if path.is_symlink() or not path.is_file():
            raise ValueError(f"Fuente ausente o no válida: {path}")
    return sorted(files, key=lambda path: path.relative_to(root).as_posix())


def snapshot(root, destination, version, epoch):
    with destination.open("xb") as output:
        with gzip.GzipFile(fileobj=output, mode="wb", filename="", mtime=epoch) as compressed:
            with tarfile.open(fileobj=compressed, mode="w", format=tarfile.PAX_FORMAT) as archive:
                for path in source_files(root):
                    name = f"skylauncher-{version}/{path.relative_to(root).as_posix()}"
                    info = archive.gettarinfo(str(path), arcname=name)
                    info.uid = info.gid = 0
                    info.uname = info.gname = ""
                    info.mtime = epoch
                    info.mode = (
                        0o755 if path.name in ("build-local.py", "build-windows.sh") else 0o644
                    )
                    with path.open("rb") as source:
                        archive.addfile(info, source)


def prepare(root, output, epoch):
    version = tomllib.loads((root / "Cargo.toml").read_text())["package"]["version"]
    if not re.fullmatch(r"[0-9][0-9A-Za-z._+]*", version):
        raise ValueError("La versión no es válida para pkgver de Arch.")
    recipe = (root / "packaging/arch/PKGBUILD").read_text()
    if f"\npkgver={version}\n" not in recipe:
        raise ValueError("Actualiza pkgver en packaging/arch/PKGBUILD para que coincida con Cargo.toml.")
    output.mkdir(parents=True, exist_ok=False)
    source = output / f"skylauncher-{version}.tar.gz"
    snapshot(root, source, version, epoch)
    digest = hashlib.sha256(source.read_bytes()).hexdigest()
    checksum = "sha256sums=('SKIP')"
    if recipe.count(checksum) != 1:
        raise ValueError("La receta local debe tener exactamente una línea de checksum bootstrap.")
    (output / "PKGBUILD").write_text(recipe.replace(checksum, f"sha256sums=('{digest}')"))
    info = subprocess.check_output(["makepkg", "--printsrcinfo"], cwd=output, text=True)
    (output / ".SRCINFO").write_text(info)
    return output


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prepare-only", action="store_true", help="Crear fuentes, PKGBUILD y .SRCINFO sin compilar")
    parser.add_argument("--output-dir", type=Path, help="Directorio de salida nuevo; nunca se sobrescribe uno existente")
    parser.add_argument("makepkg_args", nargs=argparse.REMAINDER, help="Opciones de makepkg después de --")
    args = parser.parse_args()
    if shutil.which("makepkg") is None:
        parser.error("Necesitas makepkg (Arch Linux).")
    try:
        epoch = int(os.environ.get("SOURCE_DATE_EPOCH", "0"))
    except ValueError:
        parser.error("SOURCE_DATE_EPOCH debe ser un entero.")
    if not 0 <= epoch <= 0xffffffff:
        parser.error("SOURCE_DATE_EPOCH debe ser un entero entre 0 y 4294967295.")
    if args.output_dir:
        output = args.output_dir.expanduser().resolve()
    else:
        parent = ROOT / "dist"
        parent.mkdir(exist_ok=True)
        # reserve a unique name, then let prepare create its directory itself
        output = Path(tempfile.mkdtemp(prefix="arch-build-", dir=parent))
        output.rmdir()
    try:
        prepare(ROOT, output, epoch)
        print(f"Fuentes y receta verificable: {output}", flush=True)
        if not args.prepare_only:
            flags = args.makepkg_args
            if flags[:1] == ["--"]:
                flags = flags[1:]
            subprocess.run(["makepkg", *flags], cwd=output, check=True)
        print("Preparación terminada. No se ha publicado nada en AUR.", flush=True)
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"No se pudo preparar o compilar el paquete: {error}\n")


if __name__ == "__main__":
    main()
