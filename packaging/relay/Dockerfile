# The phone's relay (plan-zeca L2), from the repository's root:
#   docker build -f packaging/relay/Dockerfile -t vults-relay .
#   docker run -p 8787:8787 vults-relay
# It speaks plain HTTP: put TLS in front (a reverse proxy) or reach it over a tailnet.
FROM docker.io/library/rust:1.98-slim-bookworm AS build
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY . .
RUN cargo build --release --locked -p vults-relay --bin vults-relay

FROM docker.io/library/debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends libssl3 ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --no-create-home relay
COPY --from=build /src/target/release/vults-relay /usr/local/bin/vults-relay
USER relay
ENV VULTS_RELAY_ADDR=0.0.0.0:8787
EXPOSE 8787
ENTRYPOINT ["/usr/local/bin/vults-relay"]
