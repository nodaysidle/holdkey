#!/usr/bin/env bash
# Usage: packaging/release.sh 0.1.0
# Tags v<version>, pushes the tag, then writes the tarball's sha256 into PKGBUILD.aur.
# The repo must be public for the AUR (and for the download below without auth).
set -euo pipefail
v="${1:?version, e.g. 0.1.0}"
cd "$(dirname "$0")/.."
git tag -a "v$v" -m "holdkey $v"
git push origin "v$v"
sum=$(curl -fsSL "https://github.com/nodaysidle/holdkey/archive/refs/tags/v$v.tar.gz" | sha256sum | cut -d' ' -f1)
sed -i "s/^pkgver=.*/pkgver=$v/; s/^sha256sums=.*/sha256sums=('$sum')/" packaging/PKGBUILD.aur
echo "PKGBUILD.aur updated: $sum"
