#!/usr/bin/env bash
# NeboAI Publisher — Install Script
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
echo "Detected platform: $PLATFORM"

# Get latest release
LATEST=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')
echo "Latest version: $LATEST"

# Download binary
DOWNLOAD_URL="https://github.com/$REPO/releases/download/$LATEST/${BINARY}-${PLATFORM}"
if [ "$PLATFORM_OS" = "windows" ]; then
  DOWNLOAD_URL="${DOWNLOAD_URL}.exe"
fi

echo "Downloading $DOWNLOAD_URL..."
TMPFILE=$(mktemp)
curl -fsSL "$DOWNLOAD_URL" -o "$TMPFILE"
chmod +x "$TMPFILE"

# Install
if [ -w "$INSTALL_DIR" ]; then
  mv "$TMPFILE" "$INSTALL_DIR/$BINARY"
else
  echo "Installing to $INSTALL_DIR (requires sudo)..."
  sudo mv "$TMPFILE" "$INSTALL_DIR/$BINARY"
fi

echo ""
echo "Installed $BINARY $LATEST to $INSTALL_DIR/$BINARY"
echo ""
echo "Next steps:"
echo "  neboai auth login    # Authenticate with NeboLoop"
echo "  neboai --help        # See all commands"
echo ""

# Install skill for Claude Code (optional)
CLAUDE_SKILLS_DIR="$HOME/.claude/skills"
if [ -d "$HOME/.claude" ]; then
  echo "Claude Code detected. Install the publisher skill? [y/N]"
  read -r REPLY
  if [[ "$REPLY" =~ ^[Yy]$ ]]; then
    mkdir -p "$CLAUDE_SKILLS_DIR"
    SKILL_DIR="$CLAUDE_SKILLS_DIR/neboai"
    if [ -d "$SKILL_DIR" ]; then
      rm -rf "$SKILL_DIR"
    fi
    # Clone just the skill files (no cli source)
    git clone --depth 1 "https://github.com/$REPO.git" "$TMPFILE.repo" 2>/dev/null
    mkdir -p "$SKILL_DIR"
    cp "$TMPFILE.repo/SKILL.md" "$SKILL_DIR/"
    cp -r "$TMPFILE.repo/references" "$SKILL_DIR/"
    cp -r "$TMPFILE.repo/scripts" "$SKILL_DIR/"
    cp -r "$TMPFILE.repo/examples" "$SKILL_DIR/"
    rm -rf "$TMPFILE.repo"
    echo "Skill installed to $SKILL_DIR"
  fi
fi
