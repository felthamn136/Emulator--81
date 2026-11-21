#!/bin/sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"

appimagetool=${APPIMAGETOOL:-appimagetool}
makensis=${MAKENSIS:-makensis}
windows_linker=${WINDOWS_LINKER:-x86_64-w64-mingw32-gcc}
appdir=$(mktemp -d "${TMPDIR:-/tmp}/Emulator-81.AppDir.XXXXXX")
trap 'rm -rf "$appdir"' EXIT HUP INT TERM

mkdir -p dist
cargo build --release

mkdir -p "$appdir/usr/bin" \
  "$appdir/usr/share/applications" \
  "$appdir/usr/share/icons/hicolor/512x512/apps"
cp target/release/zx81_emulator "$appdir/usr/bin/"
cp -R assets "$appdir/usr/bin/"
cp -R rom "$appdir/usr/bin/"
cp -R "Games Library" "$appdir/usr/bin/"
cp packaging/AppRun "$appdir/AppRun"
chmod +x "$appdir/AppRun"
cp packaging/Emulator-81.desktop "$appdir/"
cp packaging/Emulator-81.desktop "$appdir/usr/share/applications/"
cp assets/bezel/tv_cabinet.png \
  "$appdir/usr/share/icons/hicolor/512x512/apps/Emulator-81.png"
cp assets/bezel/tv_cabinet.png "$appdir/Emulator-81.png"

ARCH=x86_64 "$appimagetool" "$appdir" dist/Emulator-81-x86_64.AppImage

CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="$windows_linker" \
  cargo build --release --target x86_64-pc-windows-gnu
"$makensis" "$project_root/packaging/windows-installer.nsi"
