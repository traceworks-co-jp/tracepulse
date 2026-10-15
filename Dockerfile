# syntax=docker/dockerfile:1
FROM --platform=$BUILDPLATFORM node:22-trixie-slim AS frontend
WORKDIR /build
COPY package.json package-lock.json ./
RUN npm ci
COPY frontend/src ./frontend/src
RUN npm run build

FROM rust:1-slim-trixie AS backend
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY frontend/assets ./frontend/assets
COPY --from=frontend /build/frontend/dist ./frontend/dist
RUN cargo build --release --locked && strip target/release/tracepulse

FROM debian:trixie-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && mkdir -p /data
WORKDIR /opt/tracepulse
COPY --from=backend /build/target/release/tracepulse ./tracepulse
COPY templates ./templates
COPY config/services.toml ./config/services.toml
COPY LICENSE-MIT LICENSE-APACHE THIRD_PARTY_LICENSES.html ./
COPY docker/config.toml /etc/tracepulse/config.toml
COPY docker/docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
ENV TRACEPULSE_DATA_DIR=/data \
    TRACEPULSE_WEB_HOST=0.0.0.0
EXPOSE 8080/tcp
VOLUME ["/data"]
ENTRYPOINT ["/bin/sh", "/usr/local/bin/docker-entrypoint.sh"]
CMD ["/opt/tracepulse/tracepulse", "--web"]