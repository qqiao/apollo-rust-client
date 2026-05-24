#!/bin/sh

# Ensure clean teardown of containers on exit (success or failure)
cleanup() {
  echo "Shutting down Apollo Mock Server (docker compose down)..."
  docker compose down
}
trap cleanup EXIT INT TERM

echo "Starting Apollo Mock Server (docker compose up -d)..."
if ! docker compose up -d; then
  echo "Error: Failed to start Apollo Mock Server via docker compose." >&2
  echo "Please ensure the Docker daemon is running." >&2
  exit 1
fi

# Wait for WireMock to be ready and responsive
echo "Waiting for Apollo Mock Server to be ready..."
ready=0
for i in 1 2 3 4 5; do
  if curl -s http://localhost:8080/__admin/mappings >/dev/null; then
    echo "Apollo Mock Server is ready."
    ready=1
    break
  fi
  sleep 1
done

if [ "$ready" -ne 1 ]; then
  echo "Error: Apollo Mock Server (WireMock) failed to become ready within 5 seconds." >&2
  exit 1
fi

rm -fr /tmp/apollo && \
RUST_LOG=apollo_rust_client=trace cargo test --lib -- --nocapture && \
RUST_BACKTRACE=1 wasm-pack test --node --lib -- --nocapture
