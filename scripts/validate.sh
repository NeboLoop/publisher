#!/usr/bin/env bash
# Quick validation of artifact directories without the full Rust CLI.
# Usage: validate.sh <directory>

set -euo pipefail

DIR="${1:-.}"

if [ ! -d "$DIR" ]; then
  echo "Error: Not a directory: $DIR"
  exit 1
fi

ERRORS=0

check_json() {
  local file="$1"
  if [ -f "$file" ]; then
    if ! python3 -c "import json; json.load(open('$file'))" 2>/dev/null; then
      echo "FAIL: $file is not valid JSON"
      ERRORS=$((ERRORS + 1))
    else
      echo "OK: $file"
    fi
  fi
}

check_yaml_frontmatter() {
  local file="$1"
  if [ -f "$file" ]; then
    if ! head -1 "$file" | grep -q "^---"; then
      echo "FAIL: $file missing YAML frontmatter (must start with ---)"
      ERRORS=$((ERRORS + 1))
    else
      echo "OK: $file has frontmatter"
    fi
  fi
}

# Detect type
if [ -f "$DIR/manifest.json" ] && grep -q '"artifact_type".*"app"' "$DIR/manifest.json" 2>/dev/null; then
  echo "Type: app"
  check_json "$DIR/manifest.json"
  check_json "$DIR/agent.json"
  check_yaml_frontmatter "$DIR/AGENT.md"
  if [ ! -f "$DIR/ui/index.html" ]; then
    echo "FAIL: Missing ui/index.html"
    ERRORS=$((ERRORS + 1))
  fi
elif [ -f "$DIR/plugin.json" ]; then
  echo "Type: plugin"
  check_json "$DIR/plugin.json"
  check_yaml_frontmatter "$DIR/PLUGIN.md"
  # Check for template vars
  if grep -q '{{' "$DIR/plugin.json" 2>/dev/null; then
    echo "FAIL: plugin.json contains template variables ({{ }})"
    ERRORS=$((ERRORS + 1))
  fi
elif [ -f "$DIR/agent.json" ] && { [ -f "$DIR/AGENT.md" ] || [ -f "$DIR/agent.md" ]; }; then
  echo "Type: agent"
  check_json "$DIR/agent.json"
  check_yaml_frontmatter "$DIR/AGENT.md"
elif [ -f "$DIR/SKILL.md" ] || [ -f "$DIR/skill.md" ]; then
  echo "Type: skill"
  check_yaml_frontmatter "$DIR/SKILL.md"
else
  echo "Error: Could not detect artifact type"
  exit 1
fi

if [ $ERRORS -gt 0 ]; then
  echo ""
  echo "FAILED: $ERRORS error(s) found"
  exit 1
else
  echo ""
  echo "PASSED: All checks passed"
fi
