#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/../.." && pwd)"

VERSION="${1:-${VERSION:-$(awk -F'"' '/^version =/{print $2; exit}' "$REPO_ROOT/Cargo.toml")}}"
APP_NAME="simpad-control-panel-rs"
APP_ID="net.bysb.simpad.controlpanel"
ARCH="x86_64"
MINGW_PREFIX="${MINGW_PREFIX:-/ucrt64}"
BUNDLE_NAME="${APP_NAME}-${VERSION}-windows-${ARCH}"
STAGE_DIR="$REPO_ROOT/dist/windows-stage"
BUNDLE_DIR="$STAGE_DIR/$BUNDLE_NAME"
DIST_DIR="$REPO_ROOT/dist"
OUT_ZIP="$DIST_DIR/${BUNDLE_NAME}.zip"

for tool in cargo pkg-config glib-compile-schemas gdk-pixbuf-query-loaders ldd python3; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "Missing required tool in MSYS2 UCRT64 shell: $tool" >&2
        exit 1
    fi
done

echo "==> Building release binary ($APP_NAME $VERSION for Windows UCRT64)..."
cd "$REPO_ROOT"
cargo build --release --locked

EXE_SRC="$REPO_ROOT/target/release/${APP_NAME}.exe"
if [[ ! -f "$EXE_SRC" ]]; then
    echo "Release executable not found at $EXE_SRC" >&2
    exit 1
fi

echo "==> Creating Windows portable bundle at $BUNDLE_DIR..."
rm -rf "$STAGE_DIR"
mkdir -p \
    "$BUNDLE_DIR/lib/gtk-4.0" \
    "$BUNDLE_DIR/lib/gio/modules" \
    "$BUNDLE_DIR/share/glib-2.0/schemas" \
    "$BUNDLE_DIR/share/icons/hicolor/scalable/apps"

cp "$EXE_SRC" "$BUNDLE_DIR/${APP_NAME}.exe"
strip --strip-unneeded "$BUNDLE_DIR/${APP_NAME}.exe" || true

# Helper executables required by GLib/GIO on Windows for URI launching / D-Bus
for helper in gdbus.exe gspawn-win64-helper.exe gspawn-win64-helper-console.exe; do
    if [[ -f "$MINGW_PREFIX/bin/$helper" ]]; then
        cp "$MINGW_PREFIX/bin/$helper" "$BUNDLE_DIR/$helper"
    fi
done

install -m 644 "$REPO_ROOT/LICENSE" "$BUNDLE_DIR/LICENSE.txt"
install -m 644 "$REPO_ROOT/README.md" "$BUNDLE_DIR/README.md"
install -m 644 "$REPO_ROOT/packaging/linux/${APP_ID}.svg" \
    "$BUNDLE_DIR/share/icons/hicolor/scalable/apps/${APP_ID}.svg"

echo "==> Copying GdkPixbuf loaders..."
HOST_PIXBUF_MODULEDIR="$(pkg-config --variable=gdk_pixbuf_moduledir gdk-pixbuf-2.0 2>/dev/null || true)"
if [[ -z "$HOST_PIXBUF_MODULEDIR" || ! -d "$HOST_PIXBUF_MODULEDIR" ]]; then
    HOST_PIXBUF_MODULEDIR="$(find "$MINGW_PREFIX/lib/gdk-pixbuf-2.0" -name loaders -type d 2>/dev/null | head -n 1 || true)"
fi
if [[ -z "$HOST_PIXBUF_MODULEDIR" || ! -d "$HOST_PIXBUF_MODULEDIR" ]]; then
    echo "Could not locate MSYS2 gdk-pixbuf loaders directory" >&2
    exit 1
fi

PIXBUF_ABI_VER="$(basename "$(dirname "$HOST_PIXBUF_MODULEDIR")")"
APP_PIXBUF_DIR="$BUNDLE_DIR/lib/gdk-pixbuf-2.0/$PIXBUF_ABI_VER"
APP_PIXBUF_LOADERS="$APP_PIXBUF_DIR/loaders"
mkdir -p "$APP_PIXBUF_LOADERS"
cp "$HOST_PIXBUF_MODULEDIR/"*.dll "$APP_PIXBUF_LOADERS/"

echo "==> Collecting all transitive UCRT64 DLL dependencies..."
MINGW_PREFIX_WIN="$(cygpath -m "$MINGW_PREFIX")"
python3 - "$BUNDLE_DIR" "$APP_PIXBUF_LOADERS" "$MINGW_PREFIX" "$MINGW_PREFIX_WIN" <<'PY'
import os
import re
import shutil
import subprocess
import sys

bundle_dir, loaders_dir, prefix_posix, prefix_win = sys.argv[1:5]
prefixes = []
for p in (prefix_posix, prefix_win):
    norm = p.replace("\\", "/").rstrip("/").lower() + "/"
    if norm not in prefixes:
        prefixes.append(norm)

seeds = [
    os.path.join(bundle_dir, name)
    for name in sorted(os.listdir(bundle_dir))
    if name.lower().endswith(".exe")
]
seeds.extend(
    os.path.join(loaders_dir, name)
    for name in sorted(os.listdir(loaders_dir))
    if name.lower().endswith(".dll")
)

queue = list(seeds)
visited = set()
copied = set()
line_re = re.compile(r"^\s*([^\s]+)\s*=>\s*(.+?)(?:\s+\(0x[0-9a-fA-F]+\))?\s*$")

while queue:
    target = queue.pop(0)
    if target in visited:
        continue
    visited.add(target)

    out = subprocess.check_output(["ldd", target], text=True, stderr=subprocess.STDOUT)
    for line in out.splitlines():
        if "not found" in line:
            raise SystemExit(f"Unresolved DLL dependency in {target}: {line.strip()}")
        m = line_re.match(line)
        if not m:
            continue
        resolved = m.group(2).strip()
        resolved_norm = resolved.replace("\\", "/").lower()
        if not any(resolved_norm.startswith(p) for p in prefixes):
            continue
        dest = os.path.join(bundle_dir, os.path.basename(resolved))
        if dest.lower() not in copied:
            shutil.copy2(resolved, dest)
            copied.add(dest.lower())
            queue.append(dest)

print(f"Bundled {len(copied)} DLLs into {bundle_dir}")
PY

echo "==> Generating portable GdkPixbuf loaders.cache and loaders.cache.in..."
RAW_CACHE="$(gdk-pixbuf-query-loaders "$APP_PIXBUF_LOADERS/"*.dll)"
python3 - "$RAW_CACHE" "$PIXBUF_ABI_VER" "$APP_PIXBUF_DIR" <<'PY'
import os
import re
import sys

raw_cache, abi_ver, out_dir = sys.argv[1:4]

rel_lines = []
tpl_lines = []
dll_line_re = re.compile(r'^"(.+\.dll)"\s*$')

for line in raw_cache.splitlines():
    m = dll_line_re.match(line.strip())
    if m:
        dll_base = os.path.basename(m.group(1).replace("\\", "/"))
        rel_lines.append(f'"lib\\\\gdk-pixbuf-2.0\\\\{abi_ver}\\\\loaders\\\\{dll_base}"')
        tpl_lines.append(f'"@LOADERS_DIR@\\\\{dll_base}"')
    else:
        rel_lines.append(line)
        tpl_lines.append(line)

with open(os.path.join(out_dir, "loaders.cache"), "w", encoding="utf-8", newline="\n") as fh:
    fh.write("\n".join(rel_lines) + "\n")

with open(os.path.join(out_dir, "loaders.cache.in"), "w", encoding="utf-8", newline="\n") as fh:
    fh.write("\n".join(tpl_lines) + "\n")
PY

echo "==> Compiling GSettings schemas..."
cp "$MINGW_PREFIX/share/glib-2.0/schemas/"*.xml "$BUNDLE_DIR/share/glib-2.0/schemas/"
glib-compile-schemas "$BUNDLE_DIR/share/glib-2.0/schemas"

echo "==> Bundling icon themes (Adwaita & hicolor)..."
for theme in Adwaita hicolor; do
    if [[ -d "$MINGW_PREFIX/share/icons/$theme" ]]; then
        cp -a "$MINGW_PREFIX/share/icons/$theme" "$BUNDLE_DIR/share/icons/"
    fi
done

ICON_CACHE_TOOL="$(command -v gtk4-update-icon-cache || command -v gtk-update-icon-cache || true)"
if [[ -n "$ICON_CACHE_TOOL" ]]; then
    for theme_dir in "$BUNDLE_DIR/share/icons/"*; do
        if [[ -d "$theme_dir" && -f "$theme_dir/index.theme" ]]; then
            "$ICON_CACHE_TOOL" -f -t "$theme_dir" || true
        fi
    done
fi

echo "==> Verifying isolated DLL completeness (outside MSYS2 PATH)..."
ISOLATED_PATH="$BUNDLE_DIR:/usr/bin:/c/Windows/System32:/c/Windows"
for target in "$BUNDLE_DIR/${APP_NAME}.exe" "$APP_PIXBUF_LOADERS/"*.dll; do
    LDD_OUT="$(PATH="$ISOLATED_PATH" ldd "$target")"
    if echo "$LDD_OUT" | grep -qi "not found" \
        || echo "$LDD_OUT" | grep -qFi "$MINGW_PREFIX/" \
        || echo "$LDD_OUT" | grep -qFi "$MINGW_PREFIX_WIN/"; then
        echo "Unbundled or missing DLL dependency for $target:" >&2
        echo "$LDD_OUT" | grep -i "not found\|$MINGW_PREFIX/\|$MINGW_PREFIX_WIN/" >&2 || echo "$LDD_OUT" >&2
        exit 1
    fi
done
for req_dll in libgtk-4-1.dll libadwaita-1-0.dll libgio-2.0-0.dll libgdk_pixbuf-2.0-0.dll; do
    if [[ ! -f "$BUNDLE_DIR/$req_dll" ]]; then
        echo "Expected bundled Windows DLL missing: $req_dll" >&2
        exit 1
    fi
done

mkdir -p "$DIST_DIR"
rm -f "$OUT_ZIP"

echo "==> Creating archive: $OUT_ZIP"
if command -v zip >/dev/null 2>&1; then
    (cd "$STAGE_DIR" && zip -9 -r "$OUT_ZIP" "$BUNDLE_NAME")
else
    WIN_STAGE_DIR="$(cygpath -w "$STAGE_DIR/$BUNDLE_NAME")"
    WIN_OUT_ZIP="$(cygpath -w "$OUT_ZIP")"
    powershell.exe -NoProfile -Command "Compress-Archive -Path '$WIN_STAGE_DIR' -DestinationPath '$WIN_OUT_ZIP' -CompressionLevel Optimal"
fi

ls -lh "$OUT_ZIP"
