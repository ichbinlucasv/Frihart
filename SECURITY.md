# Security

Frihart treats the web as hostile. That is the product.

## Reporting

Until a dedicated contact is published, report vulnerabilities privately
to the maintainers of this repository. Do not open a public issue for an
exploitable bug in the parser, network stack, IPC, or sandbox.

Please include:

- Frihart version / git revision
- OS and architecture
- A minimal document or request that triggers the bug
- Whether you believe it is exploitable out of process (once Phase 6
  exists) or only in-process

We will not ask you to use a third-party bounty platform.

## Threat model (current, Phase 0/1)

Today Frihart does not fetch the live web. The relevant threats are:

| Threat | Mitigation |
| --- | --- |
| Malicious profile files | Strict parsers, no `unwrap` on disk data, lockfile |
| User-assisted code exec | No download execution; no extension runtime |
| Supply-chain crates | Minimal deps, no upload/telemetry SDKs |
| Local shoulder-surfing of history | Private windows; history can be disabled or wiped |

## Threat model (target)

| Threat | Mitigation |
| --- | --- |
| Hostile HTML/CSS | Parser in Rust, no panic paths, fixture + fuzz |
| Hostile JS (Phase 7) | Process isolation, permission default, no raw fingerprint APIs |
| Tracking | Policy crate, partitioned storage, frozen UA |
| Fingerprinting | Deny or bucket surfaces; document every exception |
| Network attacker | rustls, HTTPS-only, no custom CA surprises |
| Compromised content process | Sandbox, no profile access, no raw sockets |
| Compromised network process | Cannot paint; cannot read chrome secrets beyond what it must send |

## Things we will not do "for security"

- Phone home to check a blocklist on Frihart servers
- Force a single DoH vendor
- Ship a closed binary module (DRM, widevine, etc.)
- Hide preferences that weaken protection — we document them instead

## Update policy

Frihart does not auto-update itself in Phase 0–2. Distributions and the
user update the package. Any future updater must:

- Use TLS
- Verify signatures
- Transmit no more than the version the user is asking about
- Be off by default or distro-controlled
