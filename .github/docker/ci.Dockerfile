# The pinned Rust/Miri toolchain and CI subset of mise tools are baked once.
FROM debian:trixie-slim@sha256:a99cfc517144bc59b1978475ec53b46ecabec7e43635402ee5b77cc54cd1b20a

SHELL ["/bin/bash", "-o", "pipefail", "-c"]
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends build-essential ca-certificates curl git jq nodejs npm pkg-config unzip xz-utils && rm -rf /var/lib/apt/lists/*

ENV MISE_DATA_DIR=/opt/mise CARGO_HOME=/opt/cargo RUSTUP_HOME=/opt/rustup
ENV PATH="/opt/cargo/bin:/opt/mise/shims:${PATH}"
# Local-only tools; keep this list identical to ci.yaml's workflow environment.
ENV MISE_DISABLE_TOOLS="github:colbymchenry/codegraph,github:max-sixty/worktrunk,github:cli/cli,github:j178/prek,github:nektos/act,github:woodruffw/zizmor,npm:@commitlint/cli"
WORKDIR /workspace
COPY mise.toml mise.lock rust-toolchain.toml ./
COPY .mise/locks/npm-oxfmt/ .mise/locks/npm-oxfmt/

ARG TARGETARCH
RUN set -eu; \
    case "$TARGETARCH" in \
      amd64) arch=x64; rust_target=x86_64-unknown-linux-gnu; mise_sha=2b289d1b3074e0b1d3f95bad0bd78bbc517c1a5bcb020cbc1643d260f5d1a351; rust_sha=dda7234360b7f578ca8b0ddcb80145646fa61a67c1720a5abc7051b35c9fcb71 ;; \
      arm64) arch=arm64; rust_target=aarch64-unknown-linux-gnu; mise_sha=b405a2ea062c4d9560eee0a3c45b79e53c8834cf38e956bb47ca70e0d2e7479a; rust_sha=15f6e4ce9f583b929c996c91562bad6d4454f3281de858b02cdfdef615fac433 ;; \
      *) echo "unsupported target architecture: $TARGETARCH" >&2; exit 1 ;; \
    esac; \
    curl -fsSL "https://github.com/jdx/mise/releases/download/v2026.9.14/mise-v2026.9.14-linux-${arch}" -o /usr/local/bin/mise; \
    printf '%s  %s\n' "$mise_sha" /usr/local/bin/mise | sha256sum -c -; \
    chmod +x /usr/local/bin/mise; \
    curl -fsSL "https://static.rust-lang.org/rustup/archive/1.29.1/${rust_target}/rustup-init" -o /tmp/rustup-init; \
    printf '%s  %s\n' "$rust_sha" /tmp/rustup-init | sha256sum -c -; \
    chmod +x /tmp/rustup-init; \
    /tmp/rustup-init -y --no-modify-path --profile minimal --default-toolchain none; \
    rustup toolchain install; \
    mise install --locked; \
    mise exec -- cargo miri --version
