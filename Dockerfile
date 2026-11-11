# API do HumanoBill — build multi-stage com cache de dependências (cargo-chef)
FROM rust:1-slim-bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release -p humanobill-api --recipe-path recipe.json
COPY . .
RUN cargo build --release -p humanobill-api

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 app
WORKDIR /app
COPY --from=builder /app/target/release/humanobill-api /usr/local/bin/humanobill-api
# Só as faturas pré-extraídas da demo (sem PDFs/templates)
COPY fixtures/extracted ./fixtures/extracted
USER app
ENV FIXTURES_DIR=/app/fixtures RUST_LOG=info
EXPOSE 8080
CMD ["humanobill-api"]
