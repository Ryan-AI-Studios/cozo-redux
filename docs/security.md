# CozoDB Security Assessment (Redux)

## Accepted Upstream Risks

The following vulnerabilities have been identified and accepted as low risk due to the operational context of **Ledgerful** (retired name: ChangeGuard) and **CozoDB-redux** as local CLI tools.

### 1. lru 0.12.5 (RUSTSEC-2026-0042) — LOW
- **Impact**: Stacked Borrows violation (UB under Miri).
- **Rationale**: No known exploitable security impact. Upstream lacks a compatible patched version.

### 2. Unmaintained Dependencies — LOW
- **Packages**: `adler`, `fxhash`, `instant`.
- **Rationale**: Transitive dependencies with no known CVEs.

## Migration Path
If risk elimination becomes mandatory, the planned migration path is:
- **Storage**: SQLite
- **Graph Logic**: Petgraph

---
*Note: Historical mirror of Ledgerful internals docs (path may still say `changeguard`).*
