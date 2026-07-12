# syntax=docker/dockerfile:1.6
# COPY-only runtime image. Addresses data lives in shared PostgreSQL; CI stages build/image/ in GitHub Actions.
FROM gcr.io/distroless/cc-debian13:nonroot@sha256:d3cda6e91129130d7229a1806b6a73d292ef245ab032da7851907798024cefba

WORKDIR /app

COPY --chmod=555 sigma-addresses /app/sigma-addresses

USER nonroot:nonroot

ENV PORT=8080
EXPOSE 8080

ENTRYPOINT ["/app/sigma-addresses"]
