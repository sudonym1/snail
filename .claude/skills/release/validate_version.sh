#!/usr/bin/env bash
set -euo pipefail

# Validate version format and release readiness
# Usage: ./validate_version.sh <version>
# Example: ./validate_version.sh v1.2.3

VERSION="$1"

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

error() {
    echo -e "${RED}ERROR: $1${NC}" >&2
    exit 1
}

warning() {
    echo -e "${YELLOW}WARNING: $1${NC}" >&2
}

success() {
    echo -e "${GREEN}✓ $1${NC}"
}

# Check 1: Version format matches vX.Y.Z (semantic versioning)
echo "Validating version format..."
if ! [[ "$VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    error "Version must match format vX.Y.Z (e.g., v1.2.3), got: $VERSION"
fi
success "Format is valid: $VERSION"

# Check 2: No leading zeros
echo "Checking for leading zeros..."
if [[ "$VERSION" =~ v0[0-9] ]] || [[ "$VERSION" =~ \.[0-9]*0[0-9] ]]; then
    error "Version components cannot have leading zeros: $VERSION"
fi
success "No leading zeros"

# Check 3: Git tag doesn't already exist
echo "Checking if tag already exists..."
if git rev-parse "$VERSION" >/dev/null 2>&1; then
    error "Git tag $VERSION already exists"
fi
success "Tag does not exist"

# Check 4: Extract version number (without 'v' prefix)
VERSION_NUMBER="${VERSION#v}"

# Check 5: All Cargo.toml files have the same version
echo "Checking Cargo.toml version consistency..."
CARGO_VERSIONS=$(find . -name "Cargo.toml" -type f -exec grep -h '^version = ' {} \; | sort -u)
VERSION_COUNT=$(echo "$CARGO_VERSIONS" | wc -l)

if [ "$VERSION_COUNT" -gt 1 ]; then
    error "Multiple versions found in Cargo.toml files:\n$CARGO_VERSIONS\nAll Cargo.toml files must have the same version before creating a release."
fi
success "All Cargo.toml files have consistent versions"

# Check 6: Get current version from root Cargo.toml
echo "Checking version is newer than current..."
CURRENT_VERSION=$(grep -m1 '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')

if [ -z "$CURRENT_VERSION" ]; then
    error "Could not find current version in Cargo.toml"
fi

# Compare versions (semantic versioning comparison)
compare_versions() {
    local ver1=$1
    local ver2=$2

    IFS='.' read -ra V1 <<< "$ver1"
    IFS='.' read -ra V2 <<< "$ver2"

    # Compare major
    if [ "${V1[0]}" -gt "${V2[0]}" ]; then
        return 0
    elif [ "${V1[0]}" -lt "${V2[0]}" ]; then
        return 1
    fi

    # Compare minor
    if [ "${V1[1]}" -gt "${V2[1]}" ]; then
        return 0
    elif [ "${V1[1]}" -lt "${V2[1]}" ]; then
        return 1
    fi

    # Compare patch
    if [ "${V1[2]}" -gt "${V2[2]}" ]; then
        return 0
    else
        return 1
    fi
}

if ! compare_versions "$VERSION_NUMBER" "$CURRENT_VERSION"; then
    error "New version $VERSION_NUMBER must be greater than current version $CURRENT_VERSION"
fi
success "Version $VERSION_NUMBER is newer than current $CURRENT_VERSION"

echo ""
echo -e "${GREEN}All validation checks passed!${NC}"
echo "Ready to release version $VERSION"
