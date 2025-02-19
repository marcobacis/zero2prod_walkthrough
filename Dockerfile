FROM lukemathwalker/cargo-chef:latest-rust-1.84.1 as chef

WORKDIR /app
RUN apt update && apt install lld clang -y

# Compute lock-like file for project (to enable cache)
FROM chef as planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Build dependencies and project
FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json

# Build project dependencies
RUN cargo chef cook --release --recipe-path recipe.json

# If the dependency tree is updated, the layer should be cached
COPY . .

COPY migrations migrations

ENV SQLX_OFFLINE=true
RUN cargo build --release --bin zero2prod

# Actual release container
FROM debian:bookworm-slim AS runtime

WORKDIR /app

RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \ 
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/zero2prod zero2prod 

COPY configuration configuration

ENV APP_ENVIRONMENT=production
ENTRYPOINT ["./zero2prod"]