#!/usr/bin/env bash
# NeboAI publisher installer: the neboai CLI plus the publisher skill.
#
#   curl -fsSL https://raw.githubusercontent.com/NeboLoop/publisher/main/install.sh | bash
#
# Installs from the latest GitHub release:
#   - neboai-<os>-<arch>    -> $INSTALL_DIR/neboai   (default /usr/local/bin)
#   - neboai-skill.tar.gz   -> $SKILLS_DIR/neboai    (default ~/.claude/skills)
# Set NEBOAI_VERSION=v0.2.0 to pin a release.
set -euo pipefail

REPO="NeboLoop/publisher"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
SKILLS_DIR="${SKILLS_DIR:-$HOME/.claude/skills}"
if [ -n "${NEBOAI_VERSION:-}" ]; then
  BASE="https://github.com/$REPO/releases/download/$NEBOAI_VERSION"
else
  BASE="https://github.com/$REPO/releases/latest/download"
fi

case "$(uname -s)" in
  Darwin) OS="darwin" ;;
  Linux)  OS="linux" ;;
  *) echo "Unsupported OS: $(uname -s). On Windows, use install.ps1." >&2; exit 1 ;;
esac
case "$(uname -m)" in
  arm64|aarch64) ARCH="arm64" ;;
  x86_64|amd64)  ARCH="amd64" ;;
  *) echo "Unsupported architecture: $(uname -m)" >&2; exit 1 ;;
esac
ASSET="neboai-$OS-$ARCH"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "NeboAI publisher installer ($OS-$ARCH)"
echo ""

fetch() { curl -fsSL --retry 3 "$BASE/$1" -o "$TMP/$1"; }

echo "-> Downloading..."
fetch "$ASSET"
fetch neboai-skill.tar.gz
fetch SHA256SUMS

# Verify both downloads against the release checksums.
if command -v sha256sum >/dev/null 2>&1; then SHA="sha256sum"; else SHA="shasum -a 256"; fi
(cd "$TMP" && grep -E " ($ASSET|neboai-skill\.tar\.gz)\$" SHA256SUMS | $SHA -c - >/dev/null) \
  || { echo "Checksum verification failed." >&2; exit 1; }

# ─── CLI ───
chmod +x "$TMP/$ASSET"
if [ -d "$INSTALL_DIR" ] && [ -w "$INSTALL_DIR" ]; then
  mv "$TMP/$ASSET" "$INSTALL_DIR/neboai"
elif command -v sudo >/dev/null 2>&1; then
  echo "   (sudo is needed to write to $INSTALL_DIR)"
  sudo mkdir -p "$INSTALL_DIR"
  sudo mv "$TMP/$ASSET" "$INSTALL_DIR/neboai"
else
  INSTALL_DIR="$HOME/.local/bin"
  mkdir -p "$INSTALL_DIR"
  mv "$TMP/$ASSET" "$INSTALL_DIR/neboai"
fi
echo "   neboai $("$INSTALL_DIR/neboai" --version | awk '{print $2}') installed to $INSTALL_DIR/neboai"
case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *) echo "   Add $INSTALL_DIR to your PATH to run neboai." ;;
esac

# ─── Skill ───
mkdir -p "$SKILLS_DIR"
rm -rf "$SKILLS_DIR/neboai"
tar -xzf "$TMP/neboai-skill.tar.gz" -C "$SKILLS_DIR"
echo "   Publisher skill installed to $SKILLS_DIR/neboai"

echo ""
echo "Next: sign in with your NeboAI account"
echo "   neboai auth login"
echo ""
echo "Then ask your AI tool, for example:"
echo "   \"I have an idea for a skill that...\""
echo "   \"Publish this to NeboAI\""
