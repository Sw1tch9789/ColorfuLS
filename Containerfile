FROM rust:1.89-bookworm AS builder

ARG VERSION=dev
ARG GIT_REVISION=unknown
ARG BUILD_DATE=unknown

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY color_rules ./color_rules

RUN cargo build --release --bin cols

FROM debian:bookworm-slim AS runtime

ARG VERSION=dev
ARG GIT_REVISION=unknown
ARG BUILD_DATE=unknown

LABEL org.opencontainers.image.title="ColorfuLS CLI" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.revision="${GIT_REVISION}" \
      org.opencontainers.image.created="${BUILD_DATE}" \
      org.opencontainers.image.source="https://github.com/Sw1tch9789/ColorfuLS"

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -u 10001 appuser

WORKDIR /work

COPY --from=builder /app/target/release/cols /usr/local/bin/cols
COPY color_rules /etc/colorfuls/color_rules

ENV COLOR_RULES=/etc/colorfuls/color_rules

USER appuser

ENTRYPOINT ["cols"]