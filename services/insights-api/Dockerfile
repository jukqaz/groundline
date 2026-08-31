FROM alpine:3.23@sha256:fd791d74b68913cbb027c6546007b3f0d3bc45125f797758156952bc2d6daf40

ARG GROUNDLINE_VERSION=0.20.2
ARG GROUNDLINE_RUSTC_VERSION=unknown
ARG GROUNDLINE_BUILD_PACKAGES=unknown
ARG TARGETARCH
ENV GROUNDLINE_VERSION=${GROUNDLINE_VERSION}
LABEL org.opencontainers.image.title="GroundLine Insights API" \
      org.opencontainers.image.description="Self-hosted strict-allowlist GroundLine Insights ingest API" \
      org.opencontainers.image.source="https://github.com/jukqaz/groundline" \
      org.opencontainers.image.version="${GROUNDLINE_VERSION}" \
      org.opencontainers.image.rust.version="${GROUNDLINE_RUSTC_VERSION}" \
      org.opencontainers.image.build.packages="${GROUNDLINE_BUILD_PACKAGES}"

RUN addgroup -S -g 65532 groundline \
    && adduser -S -D -H -u 65532 -G groundline groundline

COPY image-context/groundline-insights-api-${TARGETARCH} /usr/local/bin/groundline-insights-api

USER 65532:65532
EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
  CMD ["wget", "-q", "-O", "/dev/null", "http://127.0.0.1:8080/healthz"]

ENTRYPOINT ["groundline-insights-api"]
