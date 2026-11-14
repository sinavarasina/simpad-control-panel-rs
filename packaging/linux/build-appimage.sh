#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/../.." && pwd)"

STAGE_ONLY=0
if [[ "${1:-}" == "--stage-only" ]]; then
    STAGE_ONLY=1
    shift
fi

VERSION="${1:-${VERSION:-$(awk -F'"' '/^version =/{print $2; exit}' "$REPO_ROOT/Cargo.toml")}}"
APP_NAME="simpad-control-panel-rs"
APP_ID="net.bysb.simpad.controlpanel"
ARCH="${ARCH:-x86_64}"
APPDIR="$REPO_ROOT/AppDir"
DIST_DIR="$REPO_ROOT/dist"
OUT_APPIMAGE="$DIST_DIR/${APP_NAME}-${VERSION}-${ARCH}.AppImage"

for tool in cargo patchelf desktop-file-validate glib-compile-schemas pkg-config python3; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "Missing required tool: $tool" >&2
        exit 1
    fi
done

echo "==> Building release binary ($APP_NAME $VERSION)..."
cd "$REPO_ROOT"
cargo build --release --locked

BIN_SRC="$REPO_ROOT/target/release/$APP_NAME"
if [[ ! -x "$BIN_SRC" ]]; then
    echo "Release binary not found at $BIN_SRC" >&2
    exit 1
fi

echo "==> Preparing AppDir at $APPDIR..."
rm -rf "$APPDIR"
mkdir -p \
    "$APPDIR/usr/bin" \
    "$APPDIR/usr/lib/gtk-4.0" \
    "$APPDIR/usr/lib/gio/modules" \
    "$APPDIR/usr/share/glib-2.0/schemas" \
    "$APPDIR/usr/share/icons/hicolor/scalable/apps" \
    "$APPDIR/usr/share/applications" \
    "$APPDIR/usr/share/doc/$APP_NAME"

cp "$BIN_SRC" "$APPDIR/usr/bin/$APP_NAME"
strip --strip-unneeded "$APPDIR/usr/bin/$APP_NAME" || true

install -m 755 "$SCRIPT_DIR/AppRun" "$APPDIR/AppRun"
install -m 644 "$SCRIPT_DIR/${APP_ID}.desktop" "$APPDIR/${APP_ID}.desktop"
install -m 644 "$SCRIPT_DIR/${APP_ID}.desktop" "$APPDIR/usr/share/applications/${APP_ID}.desktop"
install -m 644 "$SCRIPT_DIR/${APP_ID}.svg" "$APPDIR/${APP_ID}.svg"
install -m 644 "$SCRIPT_DIR/${APP_ID}.svg" "$APPDIR/usr/share/icons/hicolor/scalable/apps/${APP_ID}.svg"
install -m 644 "$REPO_ROOT/udev/99-simpad.rules" "$APPDIR/usr/share/doc/$APP_NAME/99-simpad.rules"
install -m 644 "$REPO_ROOT/LICENSE" "$APPDIR/usr/share/doc/$APP_NAME/LICENSE"

desktop-file-validate "$APPDIR/${APP_ID}.desktop"

echo "==> Copying GdkPixbuf loaders..."
HOST_PIXBUF_MODULEDIR="$(pkg-config --variable=gdk_pixbuf_moduledir gdk-pixbuf-2.0 2>/dev/null || true)"
if [[ -z "$HOST_PIXBUF_MODULEDIR" || ! -d "$HOST_PIXBUF_MODULEDIR" ]]; then
    HOST_PIXBUF_MODULEDIR="$(find /usr/lib /usr/lib64 -path '*/gdk-pixbuf-2.0/*/loaders' -type d 2>/dev/null | head -n 1 || true)"
fi
if [[ -z "$HOST_PIXBUF_MODULEDIR" || ! -d "$HOST_PIXBUF_MODULEDIR" ]]; then
    echo "Could not locate host gdk-pixbuf loaders directory" >&2
    exit 1
fi
PIXBUF_ABI_VER="$(basename "$(dirname "$HOST_PIXBUF_MODULEDIR")")"
APP_PIXBUF_DIR="$APPDIR/usr/lib/gdk-pixbuf-2.0/$PIXBUF_ABI_VER"
APP_PIXBUF_LOADERS="$APP_PIXBUF_DIR/loaders"
mkdir -p "$APP_PIXBUF_LOADERS"
cp -L "$HOST_PIXBUF_MODULEDIR/"*.so "$APP_PIXBUF_LOADERS/"

echo "==> Bundling transitive shared libraries (filtered by excludelist.txt)..."
python3 - "$APPDIR/usr/bin/$APP_NAME" "$APP_PIXBUF_LOADERS" "$APPDIR/usr/lib" "$SCRIPT_DIR/excludelist.txt" <<'PY'
import fnmatch
import os
import re
import shutil
import subprocess
import sys

bin_path, loaders_dir, lib_dest, exclude_file = sys.argv[1:5]

patterns = []
with open(exclude_file, "r", encoding="utf-8") as fh:
    for line in fh:
        line = line.strip()
        if line and not line.startswith("#"):
            patterns.append(line)

def is_excluded(basename: str) -> bool:
    return any(fnmatch.fnmatch(basename, pat) for pat in patterns)

queue = [bin_path]
for name in sorted(os.listdir(loaders_dir)):
    if name.endswith(".so"):
        queue.append(os.path.join(loaders_dir, name))

visited_elf = set()
copied_libs = {}
line_re = re.compile(r"^\s*([^\s]+)\s*=>\s*(/[^\s]+)\s+\(0x[0-9a-fA-F]+\)$")

while queue:
    elf = queue.pop(0)
    if elf in visited_elf:
        continue
    visited_elf.add(elf)

    out = subprocess.check_output(["ldd", elf], text=True, stderr=subprocess.STDOUT)
    for line in out.splitlines():
        if "not found" in line:
            raise SystemExit(f"Unresolved shared dependency in {elf}: {line.strip()}")
        m = line_re.match(line)
        if not m:
            continue
        soname, resolved = m.group(1), m.group(2)
        base = os.path.basename(soname)
        if is_excluded(base):
            continue
        real = os.path.realpath(resolved)
        dest = os.path.join(lib_dest, base)
        if base not in copied_libs:
            shutil.copy2(real, dest)
            os.chmod(dest, 0o755)
            copied_libs[base] = real
            queue.append(dest)

print(f"Bundled {len(copied_libs)} shared libraries into {lib_dest}")
PY

echo "==> Setting portable RPATH via patchelf..."
patchelf --set-rpath '$ORIGIN/../lib' "$APPDIR/usr/bin/$APP_NAME"
for so in "$APPDIR/usr/lib/"*.so*; do
    [[ -f "$so" ]] && patchelf --set-rpath '$ORIGIN' "$so" || true
done
for loader in "$APP_PIXBUF_LOADERS/"*.so; do
    [[ -f "$loader" ]] && patchelf --set-rpath '$ORIGIN/../../..' "$loader" || true
done

echo "==> Generating GdkPixbuf loaders cache template..."
QUERY_LOADERS="$(command -v gdk-pixbuf-query-loaders || find /usr/lib /usr/lib64 -name 'gdk-pixbuf-query-loaders' -type f 2>/dev/null | head -n 1 || true)"
if [[ -z "$QUERY_LOADERS" ]]; then
    echo "Could not find gdk-pixbuf-query-loaders" >&2
    exit 1
fi
LD_LIBRARY_PATH="$APPDIR/usr/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" \
    GDK_PIXBUF_MODULEDIR="$APP_PIXBUF_LOADERS" \
    "$QUERY_LOADERS" "$APP_PIXBUF_LOADERS/"*.so \
    | sed "s|$APPDIR|__APPDIR__|g" \
    > "$APP_PIXBUF_DIR/loaders.cache.in"

if ! grep -q "image/svg+xml" "$APP_PIXBUF_DIR/loaders.cache.in"; then
    echo "Error: SVG pixbuf loader not detected in loaders.cache.in" >&2
    exit 1
fi

echo "==> Compiling GSettings schemas..."
cp /usr/share/glib-2.0/schemas/*.xml "$APPDIR/usr/share/glib-2.0/schemas/"
glib-compile-schemas "$APPDIR/usr/share/glib-2.0/schemas"

echo "==> Bundling icon themes (Adwaita & hicolor)..."
for theme in Adwaita hicolor; do
    if [[ -d "/usr/share/icons/$theme" ]]; then
        cp -a "/usr/share/icons/$theme" "$APPDIR/usr/share/icons/"
    fi
done

ICON_CACHE_TOOL="$(command -v gtk4-update-icon-cache || command -v gtk-update-icon-cache || true)"
if [[ -n "$ICON_CACHE_TOOL" ]]; then
    for theme_dir in "$APPDIR/usr/share/icons/"*; do
        if [[ -d "$theme_dir" && -f "$theme_dir/index.theme" ]]; then
            "$ICON_CACHE_TOOL" -f -t "$theme_dir" || true
        fi
    done
fi

echo "==> Verifying AppDir library resolution..."
LD_LIBRARY_PATH="$APPDIR/usr/lib" ldd "$APPDIR/usr/bin/$APP_NAME" | grep -E "not found" && exit 1 || true
for libcheck in libgtk-4.so.1 libadwaita-1.so.0 libgio-2.0.so.0 libgdk_pixbuf-2.0.so.0; do
    if [[ ! -f "$APPDIR/usr/lib/$libcheck" ]]; then
        echo "Expected bundled library missing: $libcheck" >&2
        exit 1
    fi
done

if [[ "$STAGE_ONLY" -eq 1 ]]; then
    echo "==> Stage-only complete: $APPDIR"
    exit 0
fi

APPIMAGETOOL_BIN="${APPIMAGETOOL:-}"
if [[ -z "$APPIMAGETOOL_BIN" ]]; then
    if command -v appimagetool >/dev/null 2>&1; then
        APPIMAGETOOL_BIN="$(command -v appimagetool)"
    else
        echo "Missing appimagetool binary (set APPIMAGETOOL=/path/to/appimagetool)" >&2
        exit 1
    fi
fi

mkdir -p "$DIST_DIR"
rm -f "$OUT_APPIMAGE"

echo "==> Building AppImage: $OUT_APPIMAGE"
ARCH="$ARCH" APPIMAGE_EXTRACT_AND_RUN=1 "$APPIMAGETOOL_BIN" "$APPDIR" "$OUT_APPIMAGE"
chmod +x "$OUT_APPIMAGE"
ls -lh "$OUT_APPIMAGE"
