# Restless release Rust binaries — compiled once for every image that ships them.
#
# The account plane needs restlessd and restless; the company image needs restless and
# restless-runtime-bridge. A release compiles all three in one cargo invocation with every
# builder thread, then passes them to both images (RUST_BINARIES=prebuilt). Each image's own
# build stage stays the default, so a plain `docker build` of either image is unchanged.
#
# The toolchain block matches infra/account-plane/Dockerfile's build stage and
# infra/company-image/Dockerfile's cli stage exactly: the same base, cross compilers and target.
FROM --platform=$BUILDPLATFORM rust:1-bookworm@sha256:59037199c44290f2befcdd58dcc540164763fc296950255aaefeef096a1866b0 AS build
ARG BUILDARCH
ARG TARGETARCH
ARG CARGO_BUILD_JOBS=2
ENV CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS}
RUN set -eu; \
    case "${TARGETARCH}" in \
        amd64) compiler=gcc-x86-64-linux-gnu; sysroot=libc6-dev-amd64-cross; target=x86_64-unknown-linux-gnu ;; \
        arm64) compiler=gcc-aarch64-linux-gnu; sysroot=libc6-dev-arm64-cross; target=aarch64-unknown-linux-gnu ;; \
        *) echo "unsupported Rust image architecture: ${TARGETARCH}" >&2; exit 1 ;; \
    esac; \
    if [ "${BUILDARCH}" = "${TARGETARCH}" ]; then compiler=gcc; sysroot=libc6-dev; fi; \
    : > /rust-env; \
    if [ "${BUILDARCH}" != "${TARGETARCH}" ]; then \
        cross="$(printf '%s' "$target" | sed 's/-unknown-linux-gnu//')-linux-gnu-gcc"; \
        upper="$(printf '%s' "$target" | tr 'a-z-' 'A-Z_')"; \
        lower="$(printf '%s' "$target" | tr '-' '_')"; \
        printf 'export CARGO_TARGET_%s_LINKER=%s CC_%s=%s\n' "$upper" "$cross" "$lower" "$cross" > /rust-env; \
    fi; \
    apt-get update; \
    apt-get install --yes --no-install-recommends "$compiler" "$sysroot"; \
    rm -rf /var/lib/apt/lists/*; \
    rustup target add "$target"; \
    printf '%s' "$target" > /rust-target

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY tools/codex-runner/restless-codex-runner.mjs tools/codex-runner/gpt-6-sol-0.156.1-model-catalog.json ./tools/codex-runner/
COPY tools/custom-harness/manage.mjs tools/custom-harness/presets.json tools/custom-harness/hermes.py tools/custom-harness/run.mjs tools/custom-harness/legacy-models.mjs tools/custom-harness/openclaw.mjs tools/custom-harness/setup.mjs ./tools/custom-harness/
COPY tools/harness-auth/auth.mjs ./tools/harness-auth/auth.mjs
COPY tools/schedule-test-proxy.py ./tools/schedule-test-proxy.py
# restlessd include_str!s this at compile time (crates/restless-engine/src/context.rs:21).
COPY docs/COMPANY_OPERATING_RULES.md ./docs/COMPANY_OPERATING_RULES.md

# Stamped into restlessd only (crates/restlessd/build.rs); the other binaries do not read it.
ARG SOURCE_REVISION=""
ENV RESTLESS_SOURCE_REVISION=${SOURCE_REVISION}
# Its own target cache: one invocation unifies dependency features across the three packages,
# so its artifacts differ from the per-image builds' cache.
RUN --mount=type=cache,id=restless-cargo-registry-v1,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=restless-cargo-release-${BUILDARCH}-${TARGETARCH}-v1,target=/src/target,sharing=locked \
    target="$(cat /rust-target)" \
    && . /rust-env \
    && cargo build --release --locked --target "$target" -p restlessd -p restless -p restless-runtime-bridge \
    && for binary in restlessd restless restless-runtime-bridge; do \
         install -Dm755 "/src/target/$target/release/$binary" "/out/$binary"; \
       done

FROM scratch
COPY --from=build /out /out
