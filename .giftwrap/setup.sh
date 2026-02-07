#!/bin/bash

set -eux

export DEBIAN_FRONTEND=noninteractive
export RUSTUP_HOME=/rust/rustup
export CARGO_HOME=/rust/cargo
export PATH="${CARGO_HOME}/bin:/usr/bin:/bin:/usr/sbin:/sbin"
RUST_VERSION=1.93.0

APTGET="apt-get -o APT::Sandbox::User=root"
$APTGET update

$APTGET install -y --no-install-recommends \
    ca-certificates \
    curl \
    build-essential \
    sudo \
    neovim

ln -sf $(which nvim) /usr/local/bin/vim

curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none \
 && rustup toolchain install ${RUST_VERSION} --profile minimal \
      --component rustfmt \
      --component clippy \
      --component rust-src \
 && rustup default ${RUST_VERSION} \
 && rustc --version \
 && cargo --version \
 && cargo fmt --version
chmod -R a+rwX /rust
echo 'export PATH="/rust/cargo/bin:$PATH"' > /etc/profile.d/02-rust.sh
curl -LsSf https://astral.sh/uv/install.sh | env UV_INSTALL_DIR=/usr/local/bin INSTALLER_NO_MODIFY_PATH=1 sh
