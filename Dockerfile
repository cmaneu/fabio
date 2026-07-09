# syntax=docker/dockerfile:1

# Production Dockerfile — uses pre-built static binaries from CI.
# Expects binaries placed in binaries/{amd64,arm64}/fabio by the release
# workflow before building. Produces a multi-arch scratch image (~8MB).

FROM alpine:3 AS certs
# Extract CA certificates from Alpine (we only need the cert bundle)
RUN apk add --no-cache ca-certificates

FROM scratch

# CA certificates for HTTPS (Fabric API, OneLake, Azure auth endpoints)
COPY --from=certs /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

# ARG TARGETARCH is automatically set by Docker Buildx (amd64, arm64, etc.)
ARG TARGETARCH

# Copy the pre-built static binary for the target architecture
COPY binaries/${TARGETARCH}/fabio /usr/local/bin/fabio

USER 65534

ENTRYPOINT ["fabio"]
