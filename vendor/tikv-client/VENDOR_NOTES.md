# tikv-client 0.4.0 — Vendor Notes

Vendored: 2026-06-11
Upstream: https://github.com/tikv/client-rust tag v0.4.0
Patch: tonic 0.10 -> 0.11 (fixes RUSTSEC-2026-0098/0099/0104 and RUSTSEC-2025-0134)

Remaining work: tonic 0.12 upgrade requires proto regeneration (proto-build crate).
Review quarterly for new advisories against tikv-client 0.4.0 deps.
