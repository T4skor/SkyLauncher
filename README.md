<div align="center">

<img src="packaging/arch/skylauncher.svg" width="112" alt="SkyLauncher icon">

# SkyLauncher

**Your Minecraft, your way.**

An independent Minecraft Java Edition launcher with a block-inspired interface.
Built with Rust and a Python engine. UI language: Spanish.

![Rust](https://img.shields.io/badge/Rust-UI-7aaae2?style=flat-square)
![Python](https://img.shields.io/badge/Python-engine-74c96b?style=flat-square)
![Arch Linux](https://img.shields.io/badge/Arch-x86__64-f2c14e?style=flat-square)
![License](https://img.shields.io/badge/License-MIT-8b9bb3?style=flat-square)

</div>

## What you can do

- Play with local profiles or a Microsoft account; each new local profile has its own worlds and mods.
- Install Minecraft versions, snapshots, and Fabric, Forge, NeoForge, or Quilt where supported.
- Browse and manage mods from Modrinth, or import your own JAR files.
- Preview and save skins. Compatible loaders can enable local skins through CustomSkinLoader; SkinsRestorer servers can use skins published through MineSkin.

> [!NOTE]
> Local profiles do not grant access to Realms or authenticated servers. Local skins are visible on your own client only unless supported by a server or plugin. Publishing a skin to MineSkin makes its texture URL public.

## Download prebuilt releases

Download the files from [GitHub Releases](https://github.com/T4skor/SkyLauncher/releases/latest). You do **not** need Rust, Cargo, or a compiler to install them.

- **Windows 10/11 x64:** Download `SkyLauncher-0.3.0-windows-x64-setup.exe` and run it. Alternatively, unzip `SkyLauncher-0.3.0-windows-x64.zip` and run `SkyLauncher.exe` from the extracted folder (keep its `python/` folder beside it). No separate Python installation is needed. The binaries are unsigned, so Windows SmartScreen may warn you.
- **Arch Linux x86_64:** Download both `skylauncher-0.3.0-4-x86_64.pkg.tar.zst` and `python-minecraft-launcher-lib-8.0-1.1-any.pkg.tar.zst` from the same release. Install them together so pacman resolves the remaining system dependencies:

  ```bash
  sudo pacman -U ./python-minecraft-launcher-lib-8.0-1.1-any.pkg.tar.zst ./skylauncher-0.3.0-4-x86_64.pkg.tar.zst
  skylauncher
  ```

The Arch package uses system Python and a system keyring; a Java runtime compatible with your Minecraft version may also be needed. Packages target Arch Linux, not other Linux distributions. SkyLauncher is **not on AUR**.

## Build on Arch Linux · x86_64

If you prefer building the package yourself (requires `yay` for one Python dependency):

```bash
sudo pacman -S --needed git base-devel rust desktop-file-utils
yay -S --needed python-minecraft-launcher-lib
git clone https://github.com/T4skor/SkyLauncher.git
cd SkyLauncher
python packaging/arch/build-local.py --output-dir "$PWD/dist/arch-build-local"
sudo pacman -U dist/arch-build-local/skylauncher-0.3.0-4-x86_64.pkg.tar.zst
skylauncher
```

Use a new output directory if `dist/arch-build-local` already exists. If the AUR dependency fails its checksum check, see the [verified local alternative](docs/arch-package.md#if-falla-python-minecraft-launcher-lib-en-yay). No project virtual environment is needed after installing the package. A compatible Java runtime may be required to run Minecraft.

## Run from source

Requires Rust/Cargo, Python 3.10+, and a graphical Linux session with OpenGL and X11 or Wayland:

```bash
git clone https://github.com/T4skor/SkyLauncher.git
cd SkyLauncher
python3 -m venv .venv
.venv/bin/python -m pip install -r requirements.txt
cargo run --release --locked
```

For Windows packaging, see [the build guide](docs/windows-build.md). Microsoft sign-in requires your own registered Client ID and may require Minecraft API approval; see [session setup](docs/microsoft-session.md).

## Documentation

[Arch packaging](docs/arch-package.md) · [Profiles, mods & skins](docs/loaders-profiles-skins.md) · [Microsoft sessions](docs/microsoft-session.md)

SkyLauncher is not affiliated with Mojang or Microsoft. Source code: MIT. Bundled Tektur font: OFL-1.1.
