#!/usr/bin/env bash
# Build SkyLauncher.exe, a portable ZIP and an NSIS installer for Windows x64.
# The launcher embeds backend.py; the bundle ships an embeddable Python runtime
# so no Python installation is needed on the target machine.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERSION="$(python3 - "$ROOT/Cargo.toml" <<'PY'
import sys
import tomllib

with open(sys.argv[1], "rb") as source:
    print(tomllib.load(source)["package"]["version"])
PY
)"
if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "La versión de Cargo.toml debe ser numérica (x.y.z) para el instalador." >&2
    exit 1
fi

PY_VERSION="3.12.10"
PY_URL="https://www.python.org/ftp/python/${PY_VERSION}/python-${PY_VERSION}-embed-amd64.zip"
PY_SHA256="4acbed6dd1c744b0376e3b1cf57ce906f9dc9e95e68824584c8099a63025a3c3"
LLVM_VERSION="20261104"
LLVM_URL="https://github.com/mstorsjo/llvm-mingw/releases/download/${LLVM_VERSION}/llvm-mingw-${LLVM_VERSION}-ucrt-ubuntu-22.04-x86_64.tar.xz"
LLVM_SHA256="919380403de87dca3c581c2053230cd298af5e891286628ba0613b7fed370926"
NSIS_VERSION="3.13-1"
NSIS_DEB_URL="https://deb.debian.org/debian/pool/main/n/nsis/nsis_${NSIS_VERSION}_amd64.deb"
NSIS_DEB_SHA256="12a93aea3d2ee7f54c6c6449cc0b72ed7fbf7de8f53b639974a0a983175c5f06"
NSIS_COMMON_URL="https://deb.debian.org/debian/pool/main/n/nsis/nsis-common_${NSIS_VERSION}_all.deb"
NSIS_COMMON_SHA256="5d66cf54a09214dd7874a038a94af42894066d1fb136d662619528b0c7ed1c8c"

TARGET="x86_64-pc-windows-gnullvm"
CACHE="$ROOT/dist/cache"
STAGE="$ROOT/dist/windows/SkyLauncher"
OUT_ZIP="$ROOT/dist/SkyLauncher-${VERSION}-windows-x64.zip"
OUT_SETUP="$ROOT/dist/SkyLauncher-${VERSION}-windows-x64-setup.exe"

for tool in curl python3 cargo unzip tar ar sha256sum; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "Falta $tool. Instálalo y vuelve a intentarlo." >&2
        exit 1
    }
done
mkdir -p "$CACHE"

fetch() {
    local url="$1" checksum="$2" destination="$3"
    if [[ ! -f "$destination" ]]; then
        echo "Descargando $(basename "$destination")…"
        curl -fL --retry 3 --proto '=https' --tlsv1.2 -o "$destination.part" "$url"
        printf '%s  %s\n' "$checksum" "$destination.part" | sha256sum -c -
        mv "$destination.part" "$destination"
    else
        printf '%s  %s\n' "$checksum" "$destination" | sha256sum -c - >/dev/null
    fi
}

# llvm-mingw provides the Windows linker and UCRT sysroot for the gnullvm target.
LLVM_DIR="${LLVM_MINGW:-$CACHE/llvm-mingw-$LLVM_VERSION}"
if [[ ! -x "$LLVM_DIR/bin/x86_64-w64-mingw32-clang" ]]; then
    ARCHIVE="$CACHE/llvm-mingw-$LLVM_VERSION-ucrt-ubuntu-22.04-x86_64.tar.xz"
    fetch "$LLVM_URL" "$LLVM_SHA256" "$ARCHIVE"
    rm -rf "$LLVM_DIR"
    mkdir -p "$LLVM_DIR"
    tar -xJf "$ARCHIVE" --strip-components=1 -C "$LLVM_DIR"
fi
export PATH="$LLVM_DIR/bin:$PATH"
export CARGO_TARGET_X86_64_PC_WINDOWS_GNULLVM_LINKER="$LLVM_DIR/bin/x86_64-w64-mingw32-clang"

if command -v rustup >/dev/null 2>&1; then
    if ! rustup target list --installed | grep -qx "$TARGET"; then
        rustup target add "$TARGET"
    fi
fi

# Embeddable Python: no installer, no registry entries and no admin rights.
PY_TAG="${PY_VERSION%.*}"
PY_TAG="${PY_TAG//./}"
PY_ARCHIVE="$CACHE/python-$PY_VERSION-embed-amd64.zip"
fetch "$PY_URL" "$PY_SHA256" "$PY_ARCHIVE"
rm -rf "$STAGE"
mkdir -p "$STAGE/python"
unzip -q "$PY_ARCHIVE" -d "$STAGE/python"
cat > "$STAGE/python/python$PY_TAG._pth" <<EOF
python$PY_TAG.zip
.
Lib/site-packages
import site
EOF

# Cross-install pure wheels. keyring is installed without dependencies because
# pip evaluates sys_platform against the build host and would add SecretStorage,
# which is Linux-only. pywin32-ctypes is the Windows vault dependency.
SITE="$STAGE/python/Lib/site-packages"
mkdir -p "$SITE"
pip_cross() {
    python3 -m pip install --disable-pip-version-check --no-input --quiet \
        --no-compile --no-warn-script-location --upgrade \
        --target "$SITE" --platform win_amd64 --python-version "$PY_VERSION" \
        --implementation cp --abi "cp$PY_TAG" --only-binary=:all: "$@"
}
pip_cross 'minecraft-launcher-lib==8.0' 'requests>=2.32,<3' 'pywin32-ctypes>=0.2.0' \
    'jaraco.classes' 'jaraco.context' 'jaraco.functools'
pip_cross --no-deps 'keyring>=25,<27'
rm -rf "$SITE/bin"

echo "Compilando SkyLauncher.exe (${TARGET})…"
cargo build --release --locked --target "$TARGET" --manifest-path "$ROOT/Cargo.toml"
cp "$ROOT/target/$TARGET/release/skylauncher.exe" "$STAGE/SkyLauncher.exe"
cp "$ROOT/packaging/windows/LEEME-Windows.txt" "$STAGE/LEEME-Windows.txt"

ICON=""
if command -v magick >/dev/null 2>&1; then
    ICON="$CACHE/skylauncher.ico"
    magick -background none "$ROOT/packaging/arch/skylauncher.svg" \
        -define icon:auto-resize=256,128,64,48,32,16 "$ICON"
    cp "$ICON" "$STAGE/SkyLauncher.ico"
else
    echo "ImageMagick no está instalado: el instalador usará el icono genérico." >&2
fi

echo "Creando $(basename "$OUT_ZIP")…"
rm -f "$OUT_ZIP"
python3 - "$ROOT/dist/windows" "$OUT_ZIP" <<'PY'
import pathlib
import sys
import zipfile

base, output = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
with zipfile.ZipFile(output, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
    for path in sorted(base.rglob("*")):
        if path.is_file():
            archive.write(path, path.relative_to(base).as_posix())
PY

# Prefer a native makensis. Otherwise use an extracted Debian package, which is
# portable on other Linux distributions; Wine with the Windows NSIS also works.
NSIS_DEB="$CACHE/nsis_${NSIS_VERSION}_amd64.deb"
NSIS_COMMON="$CACHE/nsis-common_${NSIS_VERSION}_all.deb"
NSIS_PORTABLE="$CACHE/nsis-portable"
MAKENSIS="${MAKENSIS:-}"
WINE_MAKENSIS=""
if [[ -z "$MAKENSIS" ]] && command -v makensis >/dev/null 2>&1; then
    MAKENSIS="$(command -v makensis)"
fi
if [[ -z "$MAKENSIS" && -n "${NSIS_DIR:-}" ]]; then
    if [[ -x "$NSIS_DIR/usr/bin/makensis" ]]; then
        MAKENSIS="$NSIS_DIR/usr/bin/makensis"
        export NSISDIR="$NSIS_DIR/usr/share/nsis"
    elif [[ -f "$NSIS_DIR/makensis.exe" ]]; then
        WINE_MAKENSIS="$NSIS_DIR/makensis.exe"
    fi
fi
if [[ -z "$MAKENSIS" && -z "$WINE_MAKENSIS" ]]; then
    echo "Preparando makensis portable…"
    fetch "$NSIS_DEB_URL" "$NSIS_DEB_SHA256" "$NSIS_DEB"
    fetch "$NSIS_COMMON_URL" "$NSIS_COMMON_SHA256" "$NSIS_COMMON"
    rm -rf "$NSIS_PORTABLE"
    mkdir -p "$NSIS_PORTABLE/deb" "$NSIS_PORTABLE/common"
    (cd "$NSIS_PORTABLE/deb" && ar x "$NSIS_DEB" && tar -xf data.tar.xz)
    (cd "$NSIS_PORTABLE/common" && ar x "$NSIS_COMMON" && tar -xf data.tar.xz)
    MAKENSIS="$NSIS_PORTABLE/deb/usr/bin/makensis"
    export NSISDIR="$NSIS_PORTABLE/common/usr/share/nsis"
fi

EST_SIZE=$(( $(du -sk "$STAGE" | cut -f1) + 4096 ))
NSIS_ARGS=(
    -DAPP_VERSION="$VERSION"
    -DAPP_VERSION_4="$VERSION.0"
    -DEST_SIZE="$EST_SIZE"
    -DSOURCE_DIR="$STAGE"
    -DOUT_FILE="$OUT_SETUP"
)
[[ -z "$ICON" ]] || NSIS_ARGS+=(-DICON_FILE="$ICON")

echo "Creando $(basename "$OUT_SETUP")…"
rm -f "$OUT_SETUP"
if [[ -n "$MAKENSIS" ]]; then
    "$MAKENSIS" "${NSIS_ARGS[@]}" "$ROOT/packaging/windows/installer.nsi"
else
    command -v wine >/dev/null 2>&1 || {
        echo "Falta wine para usar makensis.exe." >&2
        exit 1
    }
    export WINEPREFIX="${WINEPREFIX:-$CACHE/wine}" WINEDEBUG=-all
    WINE_ARGS=(
        -DAPP_VERSION="$VERSION"
        -DAPP_VERSION_4="$VERSION.0"
        -DEST_SIZE="$EST_SIZE"
        -DSOURCE_DIR="$(winepath -w "$STAGE")"
        -DOUT_FILE="$(winepath -w "$OUT_SETUP")"
    )
    [[ -z "$ICON" ]] || WINE_ARGS+=(-DICON_FILE="$(winepath -w "$ICON")")
    wine "$WINE_MAKENSIS" "${WINE_ARGS[@]}" "$(winepath -w "$ROOT/packaging/windows/installer.nsi")"
fi

echo
echo "Artefactos:"
ls -lh "$OUT_ZIP" "$OUT_SETUP"
