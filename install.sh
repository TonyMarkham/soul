#!/usr/bin/env bash
set -euo pipefail

REPO="${SOUL_REPO:-TonyMarkham/soul}"
VERSION="${SOUL_VERSION:-latest}"
ARCHIVE="${SOUL_ARCHIVE:-}"
TARGET="${SOUL_TARGET:-.}"
CONFIG_PATH="${OPENCODE_CONFIG_PATH:-$HOME/.config/opencode/opencode.json}"
SKIP_OPENCODE_CONFIG="${SOUL_SKIP_OPENCODE_CONFIG:-0}"

usage() {
  cat <<'USAGE'
Usage: install.sh [options]

Options:
  --version <tag>              Release tag to install (default: latest)
  --archive <path>             Local release archive to install instead of downloading
  --target <path>              Repository root to install .soul into (default: current directory)
  --config <path>              opencode config path (default: ~/.config/opencode/opencode.json)
  --repo <owner/repo>          GitHub repository (default: TonyMarkham/soul)
  --skip-opencode-config       Do not modify opencode config
  -h, --help                   Show this help
USAGE
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --version) VERSION="$2"; shift 2 ;;
    --archive) ARCHIVE="$2"; shift 2 ;;
    --target) TARGET="$2"; shift 2 ;;
    --config) CONFIG_PATH="$2"; shift 2 ;;
    --repo) REPO="$2"; shift 2 ;;
    --skip-opencode-config) SKIP_OPENCODE_CONFIG=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage; exit 2 ;;
  esac
done

os_name="$(uname -s)"
arch_name="$(uname -m)"

case "$os_name:$arch_name" in
  Linux:x86_64) asset="soul-linux-x64.tar.gz" ;;
  Linux:aarch64|Linux:arm64) asset="soul-linux-arm64.tar.gz" ;;
  Darwin:arm64) asset="soul-macos-arm64.tar.gz" ;;
  *)
    echo "Unsupported platform: $os_name $arch_name" >&2
    exit 1
    ;;
esac

if [ "$VERSION" = "latest" ]; then
  url="https://github.com/${REPO}/releases/latest/download/${asset}"
else
  url="https://github.com/${REPO}/releases/download/${VERSION}/${asset}"
fi

target_root="$(mkdir -p "$TARGET" && cd "$TARGET" && pwd)"
target_soul_dir="$target_root/.soul"

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

mkdir -p "$tmp_dir/extract"
if [ -n "$ARCHIVE" ]; then
  archive_path="$ARCHIVE"
  if [ ! -f "$archive_path" ]; then
    echo "Archive not found: $archive_path" >&2
    exit 1
  fi
else
  archive_path="$tmp_dir/$asset"
  curl --fail --location "$url" --output "$archive_path"
fi
tar -xzf "$archive_path" -C "$tmp_dir/extract"

if [ ! -d "$tmp_dir/extract/.soul" ]; then
  echo "Release asset did not contain .soul/: $asset" >&2
  exit 1
fi

mkdir -p "$target_soul_dir"
cp -R "$tmp_dir/extract/.soul/." "$target_soul_dir/"
chmod 0755 "$target_soul_dir/soul" "$target_soul_dir/soul-lsp" 2>/dev/null || true

if [ "$SKIP_OPENCODE_CONFIG" != "1" ]; then
  python3 - "$CONFIG_PATH" "$target_soul_dir/soul" <<'PY'
import json
import pathlib
import shutil
import sys
import time

config_path = pathlib.Path(sys.argv[1]).expanduser()
soul_bin = str(pathlib.Path(sys.argv[2]).resolve())
config_path.parent.mkdir(parents=True, exist_ok=True)

if config_path.exists() and config_path.read_text(encoding="utf-8").strip():
    try:
        data = json.loads(config_path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise SystemExit(f"Refusing to edit invalid JSON config at {config_path}: {error}")

    if not isinstance(data, dict):
        raise SystemExit(f"Refusing to edit non-object JSON config at {config_path}")

    backup_path = config_path.with_name(f"{config_path.name}.bak.{int(time.time())}")
    shutil.copy2(config_path, backup_path)
else:
    data = {}

data.setdefault("$schema", "https://opencode.ai/config.json")

mcp = data.get("mcp")
if mcp is None:
    mcp = {}
    data["mcp"] = mcp
elif not isinstance(mcp, dict):
    raise SystemExit(f"Refusing to overwrite non-object 'mcp' value in {config_path}")

mcp["soul"] = {
    "type": "local",
    "command": [soul_bin, "serve", "--root", "."],
    "enabled": True,
}

config_path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
PY
fi

echo "Installed Soul runtime to $target_soul_dir"
if [ "$SKIP_OPENCODE_CONFIG" != "1" ]; then
  echo "Updated opencode MCP config at $CONFIG_PATH"
fi
echo "Restart opencode for the MCP config change to take effect."
