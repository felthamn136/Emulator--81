#!/bin/sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"

appimagetool=${APPIMAGETOOL:-appimagetool}
wix=${WIX:-wix}
windows_linker=${WINDOWS_LINKER:-x86_64-w64-mingw32-gcc}
appdir=$(mktemp -d "${TMPDIR:-/tmp}/Membrane81.AppDir.XXXXXX")
trap 'rm -rf "$appdir"' EXIT HUP INT TERM
chmod 755 "$appdir"

mkdir -p dist
cargo build --release
target_dir=${CARGO_TARGET_DIR:-"$project_root/target"}
case "$target_dir" in
  /*) ;;
  *) target_dir="$project_root/$target_dir" ;;
esac

mkdir -p "$appdir/usr/bin" \
  "$appdir/usr/share/applications" \
  "$appdir/usr/share/icons/hicolor/512x512/apps"
cp "$target_dir/release/membrane81" "$appdir/usr/bin/"
cp -R assets "$appdir/usr/bin/"
cp -R rom "$appdir/usr/bin/"
cp -R "Games Library" "$appdir/usr/bin/"
cp packaging/AppRun "$appdir/AppRun"
chmod +x "$appdir/AppRun"
cp packaging/Membrane81.desktop "$appdir/"
cp packaging/Membrane81.desktop "$appdir/usr/share/applications/"
cp packaging/membrane81-icon.png \
  "$appdir/usr/share/icons/hicolor/512x512/apps/Membrane81.png"
cp packaging/membrane81-icon.png "$appdir/Membrane81.png"

ARCH=x86_64 "$appimagetool" "$appdir" dist/Membrane81-x86_64.AppImage

CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="$windows_linker" \
  cargo build --release --target x86_64-pc-windows-gnu
portable="$appdir/windows-portable"
mkdir -p "$portable"
cp "$target_dir/x86_64-pc-windows-gnu/release/membrane81.exe" "$portable/"
cp -R assets rom "Games Library" "$portable/"
(
  cd "$portable"
  zip -qr "$appdir/Membrane81-Windows-Portable-x86_64.zip" .
)
mv "$appdir/Membrane81-Windows-Portable-x86_64.zip" dist/
"$wix" build -arch x64 -dcl mszip -pdbtype none \
  -d "ProjectRoot=$project_root" \
  -d "WindowsBinary=$target_dir/x86_64-pc-windows-gnu/release/membrane81.exe" \
  -o "$appdir/Membrane81-Setup.msi" \
  "$project_root/packaging/windows-installer.wxs"
mv "$appdir/Membrane81-Setup.msi" dist/
"$wix" build -arch x64 -dcl mszip -pdbtype none \
  -d "Portable=1" \
  -d "ProjectRoot=$project_root" \
  -d "WindowsBinary=$target_dir/x86_64-pc-windows-gnu/release/membrane81.exe" \
  -o "$appdir/Membrane81-Portable-Setup.msi" \
  "$project_root/packaging/windows-installer.wxs"
mv "$appdir/Membrane81-Portable-Setup.msi" dist/
