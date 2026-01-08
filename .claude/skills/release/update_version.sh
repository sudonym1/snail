#!/bin/bash

set -euo pipefail

VERSION=$1

echo "Validating version format..."
if ! [[ "$VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    error "Version must match format vX.Y.Z (e.g., v1.2.3), got: $VERSION"
fi

echo "Updating Crate Versions"
find -iname Cargo.toml | xargs sed -i 's/^version = .*/version = "'${VERSION#v}'"/'
