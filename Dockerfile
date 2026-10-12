# Must match rust-toolchain.toml. 1.87 does not build this dependency tree:
# cookie_store, home and time require rustc >= 1.88.
FROM rust:1.92-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY content ./content
COPY migrations ./migrations

RUN cargo build --release --locked

FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl gnupg \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/geonetix /usr/local/bin/geonetix
COPY package.json package-lock.json ./
COPY workers ./workers

RUN npm ci --omit=dev \
    && npx playwright install --with-deps chromium

ENV RUST_LOG=info
ENV GEONETIX_BROWSER_NO_SANDBOX=1

EXPOSE 8080

CMD ["/usr/local/bin/geonetix"]
