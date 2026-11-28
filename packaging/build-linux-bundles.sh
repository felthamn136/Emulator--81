#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
  echo "Usage: $0 EMPTY_BUILD_DIRECTORY" >&2
  exit 1
fi

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
mkdir -p "$1"
work=$(CDPATH= cd -- "$1" && pwd)
for path in flatpak-build flatpak-repo snap-stage snap-debs; do
  if [ -e "$work/$path" ]; then
    echo "Build directory must not contain $path: $work" >&2
    exit 1
  fi
done
if [ "$(uname -m)" != x86_64 ]; then
  echo "These bundle definitions require an x86-64 build host." >&2
  exit 1
fi

cd "$project_root"
cargo build --release
target_dir=${CARGO_TARGET_DIR:-"$project_root/target"}
case "$target_dir" in
  /*) ;;
  *) target_dir="$project_root/$target_dir" ;;
esac
binary="$target_dir/release/membrane81"

app_id=io.github.felthamn136.Membrane81
flatpak_build="$work/flatpak-build"
flatpak build-init "$flatpak_build" "$app_id" org.gnome.Sdk org.gnome.Platform 51
mkdir -p "$flatpak_build/files/bin" \
  "$flatpak_build/files/share/applications" \
  "$flatpak_build/files/share/icons/hicolor/512x512/apps"
cp "$binary" "$flatpak_build/files/bin/"
cp -R assets rom "Games Library" "$flatpak_build/files/bin/"
cp "packaging/$app_id.desktop" "$flatpak_build/files/share/applications/"
cp packaging/membrane81-icon.png \
  "$flatpak_build/files/share/icons/hicolor/512x512/apps/$app_id.png"
flatpak build-finish --command=membrane81 --share=ipc \
  --socket=fallback-x11 --socket=wayland --socket=pulseaudio --device=dri "$flatpak_build"
flatpak build-export "$work/flatpak-repo" "$flatpak_build" stable
flatpak build-bundle --runtime-repo=https://flathub.org/repo/flathub.flatpakrepo \
  "$work/flatpak-repo" "$work/Membrane81-x86_64.flatpak" "$app_id" stable

snap_stage="$work/snap-stage"
mkdir -p "$snap_stage/meta/gui" "$snap_stage/bin" "$snap_stage/usr/bin" "$snap_stage/etc" \
  "$snap_stage/gpu-2604" "$work/snap-debs"
cp packaging/snap/snap.yaml "$snap_stage/meta/snap.yaml"
cp packaging/snap/membrane81.desktop "$snap_stage/meta/gui/"
cp packaging/membrane81-icon.png "$snap_stage/meta/gui/membrane81.png"
cp packaging/snap/membrane81 "$snap_stage/bin/"
cp packaging/snap/alsa.conf "$snap_stage/etc/"
chmod +x "$snap_stage/bin/membrane81"
cp "$binary" "$snap_stage/usr/bin/"
cp -R assets rom "Games Library" "$snap_stage/usr/bin/"
(
  cd "$work/snap-debs"
  apt-get download libxi6 xkb-data libasound2t64 libasound2-data libasound2-plugins \
    libpulse0 libsndfile1 libflac14 libvorbis0a libvorbisenc2 libopus0 libogg0 \
    libmpg123-0t64 libmp3lame0 libasyncns0 libx11-xcb1
  for package in ./*.deb; do
    dpkg-deb -x "$package" "$snap_stage"
  done
)
snap pack --check-skeleton "$snap_stage"
snap pack --filename=Membrane81-x86_64.snap "$snap_stage" "$work"

mkdir -p dist
mv "$work/Membrane81-x86_64.flatpak" dist/
mv "$work/Membrane81-x86_64.snap" dist/
echo "Flatpak and Snap bundles written to $project_root/dist/"
