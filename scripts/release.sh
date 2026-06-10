#!/usr/bin/env bash
# scripts/release.sh — Guided release workflow for fabio
# Usage: ./scripts/release.sh <version>
# Example: ./scripts/release.sh 0.22.0
set -euo pipefail

VERSION="${1:-}"
if [[ -z "$VERSION" ]]; then
  echo "Usage: $0 <version>"
  echo "Example: $0 0.22.0"
  exit 1
fi

TAG="v${VERSION}"

# Sanity checks
if ! command -v git-cliff &>/dev/null; then
  echo "ERROR: git-cliff is not installed. Run: cargo install git-cliff"
  exit 1
fi

if git rev-parse "$TAG" &>/dev/null; then
  echo "ERROR: Tag $TAG already exists."
  exit 1
fi

if [[ -n "$(git status --porcelain)" ]]; then
  echo "ERROR: Working tree is dirty. Commit or stash changes first."
  exit 1
fi

echo "==> Preparing release $TAG"
echo ""

# Step 1: Update version in Cargo.toml
CURRENT_VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
echo "  Current version: $CURRENT_VERSION"
echo "  New version:     $VERSION"
echo ""

sed -i "s/^version = \"$CURRENT_VERSION\"/version = \"$VERSION\"/" Cargo.toml

# Step 2: Update version references in docs
sed -i "s|ghcr.io/iemejia/fabio:$CURRENT_VERSION|ghcr.io/iemejia/fabio:$VERSION|g" README.md
sed -i "s|ghcr.io/iemejia/fabio:$CURRENT_VERSION|ghcr.io/iemejia/fabio:$VERSION|g" AGENTS.md

MAJOR_MINOR=$(echo "$VERSION" | cut -d. -f1,2)
OLD_MAJOR_MINOR=$(echo "$CURRENT_VERSION" | cut -d. -f1,2)
if [[ "$MAJOR_MINOR" != "$OLD_MAJOR_MINOR" ]]; then
  sed -i "s|ghcr.io/iemejia/fabio:$OLD_MAJOR_MINOR|ghcr.io/iemejia/fabio:$MAJOR_MINOR|g" README.md
  sed -i "s|ghcr.io/iemejia/fabio:$OLD_MAJOR_MINOR|ghcr.io/iemejia/fabio:$MAJOR_MINOR|g" AGENTS.md
fi

# Step 3: Commit version bump
git add Cargo.toml README.md AGENTS.md
git commit -m "chore: bump version to $VERSION"

# Step 4: Generate changelog preview
echo ""
echo "==> Changelog preview (unreleased commits):"
echo "---"
git cliff --unreleased --tag "$TAG"
echo "---"
echo ""

# Step 5: Write release notes
NOTES_FILE="/tmp/fabio-release-notes-${VERSION}.md"
echo "  Writing release notes to: $NOTES_FILE"
echo "  Please edit this file with curated release notes."
echo ""
git cliff --unreleased --tag "$TAG" > "$NOTES_FILE"
echo ""
echo "  Raw changelog written to $NOTES_FILE"
echo "  Edit it now, then press Enter to continue..."
read -r

# Step 6: Tag and push
echo "==> Tagging $TAG and pushing..."
git tag "$TAG"
git push
git push origin "$TAG"

echo ""
echo "==> Release workflow triggered. Binaries will be available in ~10-15 min."
echo "    Docker image will follow in ~45 min."
echo ""

# Step 7: Wait for release to be created, then update notes
echo "  Waiting for GitHub Release to be created..."
for i in $(seq 1 60); do
  if gh release view "$TAG" &>/dev/null 2>&1; then
    echo "  Release $TAG found. Updating release notes..."
    gh release edit "$TAG" --notes-file "$NOTES_FILE"
    echo ""
    echo "==> Done! Release notes published."
    echo "    https://github.com/iemejia/fabio/releases/tag/$TAG"
    exit 0
  fi
  sleep 30
done

echo ""
echo "  WARNING: Timed out waiting for release to appear."
echo "  Run manually when ready: gh release edit $TAG --notes-file $NOTES_FILE"
exit 1
