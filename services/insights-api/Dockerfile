# The builder intentionally follows the moving stable Rust channel.
FROM rust:alpine AS builder

RUN apk add --no-cache build-base perl
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY xtask ./xtask
RUN --mount=type=cache,id=groundline-cargo-registry,target=/usr/local/cargo/registry \
    cargo build --locked --release -p groundline-insights-api

FROM alpine:3.23@sha256:fd791d74b68913cbb027c6546007b3f0d3bc45125f797758156952bc2d6daf40

ARG GROUNDLINE_VERSION=0.20.1
ENV GROUNDLINE_VERSION=${GROUNDLINE_VERSION}
LABEL org.opencontainers.image.title="GroundLine Insights API" \
      org.opencontainers.image.description="Self-hosted strict-allowlist GroundLine Insights ingest API" \
      org.opencontainers.image.source="https://github.com/jukqaz/groundline" \
      org.opencontainers.image.version="${GROUNDLINE_VERSION}"

RUN addgroup -S -g 65532 groundline \
    && adduser -S -D -H -u 65532 -G groundline groundline

COPY --from=builder /src/target/release/groundline-insights-api /usr/local/bin/groundline-insights-api

USER 65532:65532
EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
  CMD ["wget", "-q", "-O", "/dev/null", "http://127.0.0.1:8080/healthz"]

ENTRYPOINT ["groundline-insights-api"]
