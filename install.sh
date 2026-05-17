#!/usr/bin/env bash
# NeboAI Publisher — Install Script
# Installs the neboai CLI binary AND the skill into Claude Code / Claude Desktop.
#
# Usage: curl -fsSL https://raw.githubusercontent.com/NeboLoop/publisher/main/install.sh | bash
set -euo pipefail

REPO="NeboLoop/publisher"
BINARY="neboai"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

# Detect platform
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Darwin)  PLATFORM_OS="darwin" ;;
  Linux)   PLATFORM_OS="linux" ;;
  MINGW*|MSYS*|CYGWIN*) PLATFORM_OS="windows" ;;
  *) echo "Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
  arm64|aarch64) PLATFORM_ARCH="arm64" ;;
  x86_64|amd64)  PLATFORM_ARCH="amd64" ;;
  *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

PLATFORM="${PLATFORM_OS}-${PLATFORM_ARCH}"

echo "┌─────────────────────────────────────┐"
echo "│  NeboAI Publisher Installer          │"
echo "└─────────────────────────────────────┘"
echo ""
echo "Platform: $PLATFORM"

# ─── Step 1: Install the CLI binary ───

echo ""
echo "→ Installing neboai CLI..."

LATEST=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')

DOWNLOAD_URL="https://github.com/$REPO/releases/download/$LATEST/${BINARY}-${PLATFORM}"
if [ "$PLATFORM_OS" = "windows" ]; then
  DOWNLOAD_URL="${DOWNLOAD_URL}.exe"
fi

TMPFILE=$(mktemp)
curl -fsSL "$DOWNLOAD_URL" -o "$TMPFILE"
chmod +x "$TMPFILE"

if [ -w "$INSTALL_DIR" ]; then
  mv "$TMPFILE" "$INSTALL_DIR/$BINARY"
else
  sudo mv "$TMPFILE" "$INSTALL_DIR/$BINARY"
fi

echo "  ✓ neboai $LATEST installed to $INSTALL_DIR/$BINARY"

# ─── Step 2: Install the skill into Claude Code ───

echo ""
echo "→ Installing publisher skill..."

CLAUDE_SKILLS_DIR="$HOME/.claude/skills"
SKILL_DIR="$CLAUDE_SKILLS_DIR/neboai"

# Clone the repo to get skill files
TMPDIR=$(mktemp -d)
git clone --depth 1 --quiet "https://github.com/$REPO.git" "$TMPDIR" 2>/dev/null

# Install skill
mkdir -p "$CLAUDE_SKILLS_DIR"
rm -rf "$SKILL_DIR"
mkdir -p "$SKILL_DIR"
cp "$TMPDIR/SKILL.md" "$SKILL_DIR/"
cp -r "$TMPDIR/references" "$SKILL_DIR/"
cp -r "$TMPDIR/scripts" "$SKILL_DIR/"
cp -r "$TMPDIR/examples" "$SKILL_DIR/"
rm -rf "$TMPDIR"

echo "  ✓ Skill installed to $SKILL_DIR"

# ─── Done ───

echo ""
echo "┌─────────────────────────────────────┐"
echo "│  Ready!                             │"
echo "└─────────────────────────────────────┘"
echo ""
echo "You can now publish to NeboLoop directly from Claude."
echo ""
echo "Just tell Claude what you want to build:"
echo "  \"I have an idea for an agent that...\" "
echo "  \"Build me a plugin that connects to...\" "
echo "  \"Publish this to NeboLoop\" "
echo ""
echo "Claude handles everything — building, validating,"
echo "authenticating, and publishing. You just describe your idea."
echo ""
