# OpenPencil relay locator service

This directory packages the production `POST /v1/locator` control plane. It is
separate from the blind WebSocket relay. The service verifies a current
device-DH-bound collaboration ticket, applies the configured home-region
policy, and asks an external HSM/KMS process to sign the public locator claims.
It never receives the invite capability and contains no locator private key.

## Network boundary

The Rust service speaks bounded HTTP/1 on port 8092 inside the deployment
network. Terminate public TLS at a trusted ingress, include
`nginx-http-limits.conf` once in its `http` block, and include
`nginx-location.conf` in the exact-host TLS server. The checked-in zones
enforce per-source request and connection limits before the process-wide
aggregate limiter. If a load balancer sits in front, restore client addresses
only from its fixed trusted addresses; never trust a public
`X-Forwarded-For`. The ingress checks the raw `$request_uri`, so queries and
percent-encoded path aliases cannot be normalized into the route, and it
requires a real client `Host` header instead of Nginx's fallback host. Only
exact `POST /v1/locator`, `POST /v1/pairing-code`, and
`POST /v1/pairing-code/claim` are accepted. `/healthz` returns an empty 204 and
exposes no configuration or dependency status.

The server enforces:

- at most 32 headers and a 16 KiB HTTP header buffer;
- at most 32 headers and a 64 KiB HTTP header buffer, enough for the bounded
  48 KiB collaboration-ticket envelope;
- a five-second header timeout and body timeout;
- exactly one strict `Authorization: Bearer` value;
- the collaboration-ticket maximum plus the seven-byte Bearer prefix;
- exact request and response media types;
- an exact 191-byte non-chunked request body;
- per-source limits at the TLS ingress plus bounded process-wide authenticated
  requests per second, connections, requests, and concurrent authentication
  or HSM operations;
- a five-second authentication deadline and bounded graceful shutdown;
- empty, generic failure responses with no credential, locator, socket path,
  or account logging.

The desktop request contains the region, fresh public route id/generation,
owner Noise public key, stable relay discovery id, and lifetime. Its locally
generated 32-byte capability is never sent to this service. The returned
locator is accepted by the desktop only after pinned-signature verification
and exact comparison with that pending request.

## HSM socket

Production signing uses a fixed absolute Unix-domain socket. Every operation
rechecks that the full path contains no symlink, the final node is a
non-world-writable socket owned by the configured peer UID/GID, connects with
a timeout, and authenticates the connected peer using `SO_PEERCRED` on Linux
or `getpeereid` on supported BSD/macOS systems. Unsupported Unix platforms
fail closed.

Each connection carries one fixed request and one fixed response:

```text
request (339 bytes)
  "OPLS" | version=1 | operation=1
  | key_id_len:u8 | key_id:[64, zero padded]
  | locator_canonical_signing_bytes:[268]

response (70 bytes, then EOF)
  "OPLR" | version=1 | status:u8 | signature:[64]
```

Status `0` is success and `1` is rejection. Unknown status, zero signatures,
truncation, trailing bytes, timeout, peer mismatch, and socket replacement all
fail closed. The HSM-side adapter is deployment-owned; it must authorize the
configured key id and keep the Ed25519 private key inside the HSM/KMS boundary.
There is intentionally no software production signer in this repository.

## Configuration

Required runtime environment variables (the compose file supplies the two
container paths and listen address):

- `OPENPENCIL_COLLAB_LOCATOR_HOME_REGION=cn|global`;
- `OPENPENCIL_COLLAB_LOCATOR_TICKET_POLICY_FILE`, the absolute container path
  to the public signed collaboration union-policy JSON;
- `OPENPENCIL_COLLAB_LOCATOR_HSM_SOCKET`, the absolute container path to the
  external signer socket;
- `OPENPENCIL_COLLAB_LOCATOR_HSM_KEY_ID`, a public locator key id;
- `OPENPENCIL_COLLAB_LOCATOR_HSM_PEER_UID` and
  `OPENPENCIL_COLLAB_LOCATOR_HSM_PEER_GID`.

The compose wrapper instead requires these host mount inputs:

- `OPENPENCIL_COLLAB_POLICY_HOST_FILE`, an absolute host path to the public
  signed collaboration union-policy JSON;
- `OPENPENCIL_COLLAB_HSM_SOCKET_HOST_DIR`, an absolute host directory
  containing `signer.sock`.

The compose file mounts policy and socket inputs read-only, drops all Linux
capabilities, uses the distroless non-root identity, enables a read-only root
filesystem, and does not publish port 8092 to the host.

Both Dockerfile base images are pinned by multi-architecture manifest digest,
verified from their upstream registries on 2026-07-29. Base-image updates must
resolve and review new upstream manifests/platforms, replace both
tag-plus-digest `FROM` values, rebuild with `--pull --no-cache`, and rerun the
locator validator and collaboration security gate.

Optional runtime settings are
`OPENPENCIL_COLLAB_LOCATOR_LISTEN` (default `127.0.0.1:8092`; compose uses
`0.0.0.0:8092`),
`OPENPENCIL_COLLAB_LOCATOR_POLICY_MAX_AGE_SECONDS` (`1..=3600`),
`OPENPENCIL_COLLAB_LOCATOR_HSM_TIMEOUT_MS` (`50..=5000`),
`OPENPENCIL_COLLAB_LOCATOR_MAX_AUTH_IN_FLIGHT` (`1..=256`), and
`OPENPENCIL_COLLAB_LOCATOR_RATE_PER_SECOND` (`1..=10000`). The optional
`OPENPENCIL_COLLAB_LOCATOR_LOG_LEVEL` accepts only `error`, `warn`, `info`, or
`debug` and is scoped to this crate; arbitrary dependency trace filters are
not accepted, so HTTP headers cannot be enabled through logging configuration.

Validate the fail-closed deployment invariants and resolved compose model:

```sh
deploy/collab-relay-locator/validate.sh
```

Start after provisioning the public policy file and external signer socket:

```sh
docker compose -f deploy/collab-relay-locator/compose.yaml up --build
```

For a CN home session, deploy this service and the blind relay in the CN
region. Overseas owners/guests call the CN locator/relay endpoints over
TLS/WSS. A global edge may proxy the opaque TLS/WSS flows to the CN home
services, but it must not mint a different region or replace signed endpoint
policy.
