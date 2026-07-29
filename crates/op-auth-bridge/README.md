# op-auth-bridge

`op-auth-bridge` exposes OpenPencil's authentication bridge and collaboration
ticket verification boundary.

The Rust claim types, provider trait, JWKS cache, verifier, deterministic test
fixture, and source-only fallback are public OpenPencil code licensed under the
workspace [MIT License](../../LICENSE).

Production authentication and ticket issuance are supplied by private security
infrastructure. Target-specific artifacts under `prebuilt/` are
security-sensitive application-build inputs; they are excluded from this
crate's Cargo source package and this crate is not published to a registry.
Their licensing and provenance are managed by their independent source rather
than by this crate's notice.

The committed ABI-v1 archives are compatibility artifacts. Their exact bytes
are SHA-256 pinned and their `op_auth_*` C ABI is allowlisted, but the current
archives still contain source/build paths and debug metadata. They have **not**
been retroactively stripped, encrypted, or described as obfuscated because an
in-place binary rewrite could break final linkage. Run
`tools/check-op-auth-prebuilt.sh` for the current measured audit.

The Linux and MSVC artifacts were built as C-facing Rust `staticlib` archives,
so they also contain the producing toolchain's Rust runtime. Before a Rust host
link, `build.rs` validates the original archive, rechecks that the bytes being
staged have the validated digest, and creates a private `OUT_DIR` copy in which
the equal-length `rust_eh_personality` symbol name is namespaced to
`rust_eh_personalitx`. The one-byte suffix change also preserves the sorted
MSVC linker-member index. This updates the definition and its internal
references without changing archive offsets, keeps the committed SHA/signature
as the trust anchor, and avoids a broad linker multiple-definition exception.
Malformed archives, changed bytes, and archives containing both names fail
closed.

Production ABI-v2 artifacts fail closed unless their byte hash, target, ABI,
source revision, build id, and `op-auth-hardened-v1` declaration are covered by
an Ed25519 signature rooted in `prebuilt/PROVENANCE_PUBKEY`. The private release
pipeline must rebuild with path remapping, debug stripping, a narrow C wrapper,
LTO, and its reviewed obfuscation passes before
`tools/package-op-auth-prebuilt.sh` will stage and sign new bytes. See
[`prebuilt/README.md`](prebuilt/README.md) for the artifact contract.

Encryption at rest is useful only when the decryption key stays in private CI
and plaintext exists solely in a temporary build directory. Shipping a
decryptor and key with the application adds obscurity, not a security boundary.
Production trust never depends on client artifact secrecy: signing keys and
ticket issuance remain server-side.

## Local ABI-v2 development

Developers can exercise the collaboration UI against a private ABI-v2 archive
without replacing the committed ABI-v1 compatibility artifact:

```sh
OPENPENCIL_DEV_OP_AUTH_ARCHIVE=/absolute/path/to/libop_auth.a \
OPENPENCIL_DEV_OP_AUTH_ABI_VERSION=2 \
cargo build -p op-host-desktop --features dev-op-auth-abi-v2
```

Using the override requires the feature and both variables together; enabling
the feature without either variable is a no-op so workspace `--all-features`
checks keep using the committed artifact. The archive path must be absolute,
must select a regular non-symlink file using the artifact name expected by the
current target, and is watched for changes by Cargo. The build script copies it
into Cargo's private build-output directory before linking. This override is
accepted only in Cargo's debug profile when target debug assertions are
enabled; release, release-derived, and hardened profiles reject it. It
deliberately skips release provenance only for a local, non-shipping binary.
The runtime ABI handshake and required collaboration symbols still fail
closed.

## Regional login and collaboration trust

The credential-bearing login/ticket origin and the public collaboration trust
root are separate startup inputs:

- `OPENPENCIL_SSO_URL` selects the regional SSO used for device login,
  account calls, sign-out, and `POST /api/v1/collab/tickets`;
- `OPENPENCIL_COLLAB_ISSUER` pins the exact logical `iss` accepted from every
  collaborating region;
- `OPENPENCIL_COLLAB_POLICY_ENDPOINT` optionally pins a regional HTTPS mirror
  of the offline-signed union policy;
- `OPENPENCIL_COLLAB_JWKS_ENDPOINT` selects only the explicit legacy raw-JWKS
  compatibility path for self-hosted deployments.

With an issuer but no endpoint override, the bridge derives
`/api/v1/collab/policy` on that issuer. With no overrides, production also uses
the signed policy endpoint. An explicit `OPENPENCIL_SSO_URL`-only self-hosted
configuration retains its legacy same-origin JWKS behavior. Either endpoint
override requires an explicit issuer, the two endpoint variables are mutually
exclusive, and a policy parse, signature, time, or generation failure never
falls back to raw JWKS.

Domestic and overseas sites may use different `OPENPENCIL_SSO_URL` values only
when both issuers produce the same logical `iss` and globally stable account
`sub`. Each region keeps its own HSM private keys. Every regional policy mirror
must publish the same canonical envelope signed by the offline root pinned in
this crate. The verifier requires the exact issuer, a live seven-day-or-shorter
window, at most 8 regions/24 keys, one active plus one next key per region,
globally unique `kid` and public keys, and safe overlap metadata. Next keys are
integrity-checked but cannot verify tickets before activation; expired overlap
keys fail closed. A process rejects generation rollback and same-generation
rewrites. Neither tickets, the private provider, discovery, peers, nor a
regional mirror can replace the offline root.

When no compatible private artifact is present, the crate builds its public
stub backend. Authentication availability then reports `false`, while the open
collaboration-ticket verifier and test fixture remain usable.
