# OpenPencil collaboration relay

The relay is an untrusted, blind WebSocket byte forwarder. TLS terminates at
the edge and the Rust process listens on `127.0.0.1:8091` by default. The
existing Noise session still encrypts document traffic end to end, so the
relay never receives plaintext document operations.

Only `GET /v1/tunnel` upgrades are accepted. Routing uses a domain-separated
BLAKE3 map key; capabilities, tickets, challenges, proofs, and payloads are
never logged.

## Production authentication

The standalone binary fails closed unless an explicit mode is selected.
`--production` enables signed-ticket, signed-locator, region, owner-key, and
challenge-bound X25519 proof verification.

For every successful authenticated WebSocket upgrade the relay sends exactly
one fresh:

```text
OpenPencil-Relay-Challenge: oprc1_<canonical-base64url>
```

Nginx must preserve this `101` response header.
`nginx.conf` sets `proxy_pass_header OpenPencil-Relay-Challenge` explicitly.
Strict clients reject a missing, duplicate, malformed, unknown-key, or
noncanonical challenge before sending a hello.

The checked-in public and federation TLS servers use bounded 64 KiB client
header buffers so the valid 48 KiB ticket ceiling fits in `Authorization`
without making request headers unbounded.

The challenge contains a pinned relay key id and a fresh 32-byte nonce. The
client retains the exact bearer used on that upgrade, performs X25519 with its
existing device private key and the pinned relay public key, derives a key with
HKDF-SHA256, and sends a fixed 33-byte HMAC-SHA256 proof in authentication mode
2. The proof binds:

- the complete challenge and protocol versions;
- SHA-256 of the exact bearer-token bytes;
- role and caller device public key;
- route capability and complete signed locator.

The per-connection challenge is non-cloneable server state, expires with the
bounded hello deadline, and is consumed by the first authentication attempt.
A proof replayed on another connection, used with a refreshed bearer, or moved
to another role/route/locator fails as `AuthenticationFailed`.

`RelayServerX25519ProofBoundary` is the HSM-capable verification boundary.
An HSM adapter can retain the private key and derived secret internally. The
packaged CLI also provides a sealed-file adapter for deployments without that
adapter; no private key belongs in source control or an image layer.

Blocking policy, signature, and proof operations run outside Tokio workers
behind `OPENPENCIL_COLLAB_RELAY_MAX_AUTH_IN_FLIGHT`. Its permit remains held
through challenge issuance, WebSocket upgrade, hello receipt, and actual
verification.

## Required settings

- `OPENPENCIL_COLLAB_RELAY_HOME_REGION=cn|global`
- `OPENPENCIL_COLLAB_RELAY_TICKET_POLICY_FILE`: absolute path to the pinned
  signed union-policy JSON
- `OPENPENCIL_COLLAB_RELAY_LOCATOR_KEYS_FILE`: absolute path to locator
  Ed25519 public keys
- `OPENPENCIL_COLLAB_RELAY_X25519_KEYS_FILE`: absolute path to the sealed relay
  X25519 key set
- optional `OPENPENCIL_COLLAB_RELAY_POLICY_MAX_AGE_SECONDS` (`1..=3600`,
  default `60`)
- optional `OPENPENCIL_COLLAB_RELAY_LOG_LEVEL=error|warn|info|debug` (default
  `info`); arbitrary `RUST_LOG` dependency filters and `trace` are not accepted

Locator public keys use:

```json
{
  "version": 1,
  "keys": [
    {
      "kid": "locator-key-2026-07",
      "public_key_ed25519": "canonical-base64url-no-padding"
    }
  ]
}
```

The sealed relay X25519 file uses:

```json
{
  "version": 1,
  "active_kid": "relay-pop-2026-07",
  "keys": [
    {
      "kid": "relay-pop-2026-07",
      "private_key_x25519": "canonical-base64url-no-padding",
      "public_key_x25519": "canonical-base64url-no-padding"
    }
  ]
}
```

The public value and key id are distributed to clients as pinned relay keys.
On Unix, the sealed file must be a regular non-symlink file with no group or
other permission bits. Reads are bounded and use `O_NOFOLLOW | O_CLOEXEC`.
Private encodings, shared secrets, and derived keys are zeroized.

## Deployment

Run full challenge-bound production mode:

```sh
export OPENPENCIL_COLLAB_POLICY_HOST_FILE=/secure/public/collab-policy.json
export OPENPENCIL_RELAY_LOCATOR_KEYS_HOST_FILE=/secure/public/locator-keys.json
export OPENPENCIL_RELAY_X25519_KEYS_HOST_FILE=/secure/private/relay-x25519-keys.json
sudo chown 65532:65532 "$OPENPENCIL_RELAY_X25519_KEYS_HOST_FILE"
sudo chmod 0400 "$OPENPENCIL_RELAY_X25519_KEYS_HOST_FILE"
docker compose \
  -f deploy/collab-relay/compose.yaml \
  -f deploy/collab-relay/compose.production.yaml \
  up --build
```

The secret is mounted read-only under `/run/secrets`. The packaged image runs
as UID/GID `65532`, so the host file must actually be owner-readable by that
identity and inaccessible to group/other users. Docker Compose silently
ignores `uid`, `gid`, and `mode` on file-backed secrets because they are bind
mounts; the overlay therefore does not claim those ineffective attributes.
Verify the effective ownership, mode, and readability on the target Linux
host before starting production. A key-read or permission failure must stop
the relay. Prefer a platform-managed secret or an HSM-backed
`RelayServerX25519ProofBoundary` for a long-lived Internet deployment.

Both Dockerfile base images are pinned by multi-architecture manifest digest,
verified from their upstream registries on 2026-07-29. For an update, resolve
the intended Rust and distroless tags from the upstream registries, review the
new manifests/platforms and security delta, replace both tag-plus-digest
`FROM` values, rebuild with `--pull --no-cache`, and rerun the deployment and
workspace security gates.

Clients not yet wired for authentication mode 2 may use only the separate,
explicit reduced-assurance overlay:

```sh
export OPENPENCIL_COLLAB_POLICY_HOST_FILE=/secure/public/collab-policy.json
export OPENPENCIL_RELAY_LOCATOR_KEYS_HOST_FILE=/secure/public/locator-keys.json
docker compose \
  -f deploy/collab-relay/compose.yaml \
  -f deploy/collab-relay/compose.reduced-assurance.yaml \
  up --build
```

That mode still verifies ticket-to-device-DH binding, locator signature,
region, expiry, and Owner static binding, but has no challenge proof. It
accepts only authentication mode 1 without proof and never becomes the
default.

For local capability-only development:

```sh
docker compose \
  -f deploy/collab-relay/compose.yaml \
  -f deploy/collab-relay/compose.dev.yaml \
  up --build
```

Never expose that development mode to the Internet.

## China relay for overseas peers

For a locator with `home_region=cn`, domestic clients connect to the normal CN
WSS endpoint. Overseas clients may instead resolve a Global ingress that is
only an L4 passthrough:

```text
overseas client inner TLS/WSS
  -> Global L4 stream proxy
  -> fixed outer-mTLS backhaul
  -> private CN federation listener
  -> normal CN TLS/WSS terminator
  -> the same CN relay process
```

The Global hop never terminates or inspects the inner TLS stream and is not a
Global home relay or fallback. Both peers still reach the same CN pairing
table, and the CN authenticator remains authoritative for the signed
`home_region=cn`. See
[`deploy/collab-relay-edge`](../collab-relay-edge/README.md) for the fixed
nested-TLS topology, certificates, limits, and validation steps.

On an overseas desktop, configure the logical CN endpoint
`OPENPENCIL_COLLAB_RELAY_CN_URL` with the Global ingress WSS hostname, or use
GeoDNS/split DNS so that the same CN endpoint hostname resolves to Global
ingress overseas and the direct CN terminator domestically. The inner
certificate is still presented by the CN terminator and must cover that
hostname. `OPENPENCIL_COLLAB_RELAY_GLOBAL_URL` remains reserved for locators
whose signed home region is actually `global`; it is never an implicit
fallback for a CN locator. A domestic peer and an overseas peer can therefore
take different network paths while converging on the same CN relay
process/shard.

This intentionally creates a cross-border data path and requires the
applicable transfer, latency, reachability, and availability review. A CN
relay rejects a locator signed for the Global home region.

The server currently keeps pairing state in one process. Do not round-robin a
relay hostname across independent replicas without stable route-key sharding
or a shared rendezvous layer.
