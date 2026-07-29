#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
compose_file="$script_dir/compose.yaml"
dockerfile="$script_dir/Dockerfile"
nginx_config="$script_dir/nginx-location.conf"
nginx_limits="$script_dir/nginx-http-limits.conf"

require_literal() {
    pattern=$1
    file=$2
    grep -F "$pattern" "$file" >/dev/null
}

for pattern in \
    'ENTRYPOINT ["/usr/local/bin/op-collab-relay-locator-server", "--production"]' \
    'FROM rust:1.94-bookworm@sha256:6ae102bdbf528294bc79ad6e1fae682f6f7c2a6e6621506ba959f9685b308a55 AS build' \
    'FROM gcr.io/distroless/cc-debian12:nonroot@sha256:fccdbb0a547c14e23fcf4ce8ad62ca5d43b4faae8d22cd292f490fef9946c96e' \
    'cargo build --locked --release -p op-collab-relay-locator-server'
do
    require_literal "$pattern" "$dockerfile"
done

for pattern in \
    'read_only: true' \
    'cap_drop:' \
    'no-new-privileges:true' \
    'OPENPENCIL_COLLAB_LOCATOR_TICKET_POLICY_FILE: /run/secrets/collab-policy.json' \
    'OPENPENCIL_COLLAB_LOCATOR_HSM_SOCKET: /run/openpencil-hsm/signer.sock'
do
    require_literal "$pattern" "$compose_file"
done

for pattern in \
    'location = /v1/locator {' \
    'if ($request_uri != "/v1/locator") {' \
    'if ($http_host = "") {' \
    'client_max_body_size 191;' \
    'proxy_set_header Authorization $http_authorization;' \
    'proxy_set_header Host $http_host;' \
    'proxy_buffering off;' \
    'location = /healthz {' \
    'access_log off;' \
    'location / {' \
    'return 404;'
do
    require_literal "$pattern" "$nginx_config"
done

for pattern in \
    'limit_req_zone $binary_remote_addr zone=openpencil_locator_per_source:10m rate=10r/s;' \
    'limit_conn_zone $binary_remote_addr zone=openpencil_locator_connections:10m;' \
    'client_header_buffer_size 64k;' \
    'large_client_header_buffers 2 64k;'
do
    require_literal "$pattern" "$nginx_limits"
done

for pattern in \
    'limit_req zone=openpencil_locator_per_source burst=20 nodelay;' \
    'limit_req_status 429;' \
    'limit_conn openpencil_locator_connections 16;' \
    'limit_conn_status 429;'
do
    require_literal "$pattern" "$nginx_config"
done

if grep -Eq \
    '^[[:space:]]*(ports:|privileged:|network_mode:[[:space:]]*host)' \
    "$compose_file"
then
    echo "locator container must not publish a host port or gain host privileges" >&2
    exit 1
fi

if grep -Eq \
    'proxy_pass[[:space:]]+\$|proxy_next_upstream[[:space:]]+(on|error|timeout|http_)' \
    "$nginx_config"
then
    echo "dynamic or fallback locator upstream is forbidden" >&2
    exit 1
fi

if command -v docker >/dev/null 2>&1 &&
    docker compose version >/dev/null 2>&1
then
    OPENPENCIL_COLLAB_LOCATOR_HOME_REGION=cn \
    OPENPENCIL_COLLAB_LOCATOR_HSM_KEY_ID=locator-prod-2026-07 \
    OPENPENCIL_COLLAB_LOCATOR_HSM_PEER_UID=65532 \
    OPENPENCIL_COLLAB_LOCATOR_HSM_PEER_GID=65532 \
    OPENPENCIL_COLLAB_POLICY_HOST_FILE=/dev/null \
    OPENPENCIL_COLLAB_HSM_SOCKET_HOST_DIR=/tmp \
        docker compose -f "$compose_file" config -q
fi

echo "collab relay locator deployment validation passed"
