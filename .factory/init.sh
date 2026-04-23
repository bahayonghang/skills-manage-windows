#!/bin/bash
set -e

cd /Users/happypeet/Documents/GitHubMe/skills-manage

if [ ! -d "node_modules" ]; then
  pnpm install
fi

if [ -d "src-tauri" ]; then
  cargo fetch --manifest-path src-tauri/Cargo.toml 2>/dev/null || true
fi

FIXTURE_HOME=/tmp/skills-manage-test-fixtures/claude-multi-source
mkdir -p "$FIXTURE_HOME/.claude/skills"
mkdir -p "$FIXTURE_HOME/.claude/plugins/marketplaces/marketplace-a"
mkdir -p "$FIXTURE_HOME/.claude/plugins/marketplaces/marketplace-b"
mkdir -p "$FIXTURE_HOME/.agents/skills"
mkdir -p "$FIXTURE_HOME/.skillsmanage"

echo "Init complete for Claude multi-source scan mission."
