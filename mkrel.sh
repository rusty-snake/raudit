#!/bin/bash

# Copyright © 2021 rusty-snake
#
# This file is part of raudit
#
# raudit is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# raudit is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program. If not, see <https://www.gnu.org/licenses/>.

set -euo pipefail

# cd into the project directory
cd -P -- "$(readlink -e "$(dirname "$0")")"

# Do not run if an old outdir exists
[ -d outdir ] && { echo "Please delete 'outdir' first."; exit 1; }

# Check presents of non-standard programs (everything except coreutils and built-ins)
if ! command -v cargo >&-; then
    echo "mkrel.sh: Missing requirement: cargo is not installed or could not be found."
    echo "Please make sure cargo is installed and in \$PATH."
    exit 1
fi
if ! command -v git >&-; then
    echo "mkrel.sh: Missing requirement: git is not installed or could not be found."
    echo "Please make sure git is installed and in \$PATH."
    exit 1
fi
if ! command -v podman >&-; then
    echo "mkrel.sh: Missing requirement: podman is not installed or could not be found."
    echo "Please make sure podman is installed and in \$PATH."
    exit 1
fi
if ! command -v xz >&-; then
    echo "mkrel.sh: Missing requirement: xz is not installed or could not be found."
    echo "Please make sure xz is installed and in \$PATH."
    exit 1
fi

# Pull alpine image if necessary
if [[ -z "$(podman image list --noheading alpine:latest)" ]]; then
    podman pull docker.io/library/alpine:latest
fi

# Check if we are allowed to run podman
if [[ "$(podman run --rm alpine:latest echo "hello")" != "hello" ]]; then
    echo "mkrel.sh: podman does not seem to work correctly."
    exit 1
fi

IFS='#' read -r PROJECT VERSION < <(basename "$(cargo pkgid)")

# Vendor all dependencies
cargo vendor --color=never --locked vendor
[ -d .cargo ] && mv -v .cargo .cargo.bak
mkdir -v .cargo
trap "rm -rv .cargo && [ -d .cargo.bak ] && mv -v .cargo.bak .cargo" EXIT
echo "[ INFO ] Create .cargo/config.toml"
cat > .cargo/config.toml <<-EOF
[source.crates-io]
replace-with = "vendored-sources"
[source.vendored-sources]
directory = "vendor"
EOF

mkdir -v outdir

echo "[ INFO ] Pack source archive"
git archive --format=tar --prefix="$PROJECT-$VERSION/" -o "outdir/$PROJECT-$VERSION.src.tar" HEAD
tar --xform="s,^,$PROJECT-$VERSION/," -rf "outdir/$PROJECT-$VERSION.src.tar" .cargo vendor
xz "outdir/$PROJECT-$VERSION.src.tar"
echo "[ INFO ] Finish"

echo "[ INFO ] Build binary archive"
BUILDDIR="/builddir"
INSTALLDIR="/installdir"
podman run --rm --security-opt=no-new-privileges --cap-drop=all \
    -v ./outdir:/outdir:z --tmpfs "$BUILDDIR" --tmpfs "$INSTALLDIR:mode=0755" \
    -w "$BUILDDIR" alpine:latest sh -euo pipefail -c "
        apk update
        apk upgrade ||:
        apk add curl gcc make musl-dev py3-docutils xz ||:
        curl --proto '=https' --tlsv1.3 -sSf 'https://sh.rustup.rs' | sh -s -- -y --profile minimal
        source ~/.cargo/env
        tar --strip=1 -xf '/outdir/$PROJECT-$VERSION.src.tar.xz'
        cargo build --release --frozen
        strip ./target/release/raudit
        PREFIX=/usr/local
        install -Dm0755 ./target/release/raudit '$INSTALLDIR/usr/local/libexec/raudit'
        install -Dm0644 -t '$INSTALLDIR/usr/local/share/raudit' ./share/*.rules
        install -Dm0644 ./docs/source/how-can-i-fix.rst '$INSTALLDIR/usr/local/share/doc/raudit/how-can-i-fix.rst'
        install -Dm0644 ./CHANGELOG.md '$INSTALLDIR/usr/local/share/doc/raudit/CHANGELOG.md'
        install -Dm0644 ./README.md '$INSTALLDIR/usr/local/share/doc/raudit/README.md'
        install -Dm0644 ./AUTHORS '$INSTALLDIR/usr/local/share/licenses/raudit/AUTHORS'
        install -Dm0644 ./COPYING '$INSTALLDIR/usr/local/share/licenses/raudit/COPYING'
        tar -C '$INSTALLDIR' -cJf '/outdir/$PROJECT-$VERSION-x86_64-unknown-linux-musl.tar.xz' .
    "
echo "[ INFO ] Finish"

# Compute checksums
sha256sum outdir/*.tar.xz > outdir/SHA256SUMS
sha512sum outdir/*.tar.xz > outdir/SHA512SUMS
echo "Success!"
