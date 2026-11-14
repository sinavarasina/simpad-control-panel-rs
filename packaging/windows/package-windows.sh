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

for tool in cargo pkg-config glib-compile-schemas gdk-pixbuf-query-loaders objdump cygpath python3; do
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
BUNDLE_DIR_WIN="$(cygpath -m "$BUNDLE_DIR")"
LOADERS_DIR_WIN="$(cygpath -m "$APP_PIXBUF_LOADERS")"
MINGW_BIN_WIN="$(cygpath -m "$MINGW_PREFIX/bin")"
SYS32_WIN="$(cygpath -m "${SYSTEMROOT:-C:/Windows}/System32")"

python3 - "$BUNDLE_DIR_WIN" "$LOADERS_DIR_WIN" "$MINGW_BIN_WIN" "$SYS32_WIN" <<'PY'
import os
import re
import shutil
import subprocess
import sys

bundle_dir, loaders_dir, mingw_bin, sys32_dir = sys.argv[1:5]

mingw_dlls = {
    name.lower(): os.path.join(mingw_bin, name)
    for name in os.listdir(mingw_bin)
    if name.lower().endswith(".dll")
}
sys_dlls = {
    name.lower()
    for name in os.listdir(sys32_dir)
    if name.lower().endswith((".dll", ".drv"))
}

def is_windows_virtual_dll(name_lower: str) -> bool:
    return name_lower.startswith(("api-ms-win-", "ext-ms-win-"))

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
missing = []
dll_re = re.compile(r"^\s*DLL Name:\s*(.+?)\s*$")

while queue:
    target = queue.pop(0)
    norm_target = os.path.normcase(os.path.abspath(target))
    if norm_target in visited:
        continue
    visited.add(norm_target)

    out = subprocess.check_output(["objdump", "-p", target], text=True, stderr=subprocess.STDOUT)
    for line in out.splitlines():
        m = dll_re.match(line)
        if not m:
            continue
        dep = m.group(1).strip()
        dep_lower = dep.lower()
        if dep_lower in mingw_dlls:
            src = mingw_dlls[dep_lower]
            dest = os.path.join(bundle_dir, os.path.basename(src))
            if dep_lower not in copied:
                shutil.copy2(src, dest)
                copied.add(dep_lower)
                queue.append(dest)
        elif dep_lower in sys_dlls or is_windows_virtual_dll(dep_lower):
            continue
        elif not os.path.isfile(os.path.join(bundle_dir, dep)):
            missing.append((os.path.basename(target), dep))

if missing:
    for parent, dep in missing:
        print(f"Unresolved Windows DLL dependency: {parent} -> {dep}", file=sys.stderr)
    raise SystemExit(1)

print(f"Bundled {len(copied)} DLLs into {bundle_dir}")
PY

echo "==> Generating portable GdkPixbuf loaders.cache and loaders.cache.in..."
APP_PIXBUF_DIR_WIN="$(cygpath -m "$APP_PIXBUF_DIR")"
RAW_CACHE_FILE="$APP_PIXBUF_DIR/loaders.cache.raw"
gdk-pixbuf-query-loaders "$APP_PIXBUF_LOADERS/"*.dll > "$RAW_CACHE_FILE"
RAW_CACHE_WIN="$(cygpath -m "$RAW_CACHE_FILE")"
python3 - "$RAW_CACHE_WIN" "$PIXBUF_ABI_VER" "$APP_PIXBUF_DIR_WIN" <<'PY'
import os
import re
import sys

raw_cache_path, abi_ver, out_dir = sys.argv[1:4]
with open(raw_cache_path, "r", encoding="utf-8") as fh:
    raw_cache = fh.read()

rel_lines = []
tpl_lines = []
dll_line_re = re.compile(r'^"(.+\.dll)"\s*$', re.IGNORECASE)
count = 0

for line in raw_cache.splitlines():
    m = dll_line_re.match(line.strip())
    if m:
        dll_base = os.path.basename(m.group(1).replace("\\", "/"))
        rel_lines.append(f'"lib\\\\gdk-pixbuf-2.0\\\\{abi_ver}\\\\loaders\\\\{dll_base}"')
        tpl_lines.append(f'"@LOADERS_DIR@\\\\{dll_base}"')
        count += 1
    else:
        rel_lines.append(line)
        tpl_lines.append(line)

if count == 0:
    raise SystemExit("Error: no loader entries generated in loaders.cache.in")

with open(os.path.join(out_dir, "loaders.cache"), "w", encoding="utf-8", newline="\n") as fh:
    fh.write("\n".join(rel_lines) + "\n")

with open(os.path.join(out_dir, "loaders.cache.in"), "w", encoding="utf-8", newline="\n") as fh:
    fh.write("\n".join(tpl_lines) + "\n")

print(f"Registered {count} GdkPixbuf loader(s)")
PY
rm -f "$RAW_CACHE_FILE"

if ! grep -q "image/svg+xml" "$APP_PIXBUF_DIR/loaders.cache.in"; then
    echo "Error: SVG pixbuf loader not detected in loaders.cache.in" >&2
    exit 1
fi

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

echo "==> Verifying isolated DLL completeness via PE headers..."
python3 - "$BUNDLE_DIR_WIN" "$LOADERS_DIR_WIN" "$SYS32_WIN" <<'PY'
import os
import re
import subprocess
import sys

bundle_dir, loaders_dir, sys32_dir = sys.argv[1:4]
bundled = {name.lower() for name in os.listdir(bundle_dir) if name.lower().endswith(".dll")}
sys_dlls = {
    name.lower()
    for name in os.listdir(sys32_dir)
    if name.lower().endswith((".dll", ".drv"))
}
dll_re = re.compile(r"^\s*DLL Name:\s*(.+?)\s*$")

targets = [
    os.path.join(bundle_dir, name)
    for name in sorted(os.listdir(bundle_dir))
    if name.lower().endswith((".exe", ".dll"))
]
targets.extend(
    os.path.join(loaders_dir, name)
    for name in sorted(os.listdir(loaders_dir))
    if name.lower().endswith(".dll")
)

for target in targets:
    out = subprocess.check_output(["objdump", "-p", target], text=True, stderr=subprocess.STDOUT)
    for line in out.splitlines():
        m = dll_re.match(line)
        if not m:
            continue
        dep = m.group(1).strip().lower()
        if dep in bundled or dep in sys_dlls or dep.startswith(("api-ms-win-", "ext-ms-win-")):
            continue
        raise SystemExit(f"Missing bundled DLL for {os.path.basename(target)}: {dep}")

print(f"Verified {len(targets)} PE binaries/DLLs cleanly resolved within bundle or Windows System32")
PY
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
