# [Refactor] Migrate Unmaintained `chrono` (RUSTSEC-2026-0543) to `jiff` Across Workspace

> Area: Infra / Tech Debt / Security.  
> Labels: `refactor`, `security`, `dx` (without `jules` label until pre-dispatch audit is complete)

---

## Current State (MEASURABLE)
- **Security Advisory**: RUSTSEC-2026-0543 officially categorizes `chrono` as `INFO Unmaintained` (author stepped down, repository maintenance halted).
- **Workspace References**:
  - `git grep -w "chrono" | wc -l` produces `932` occurrences across the workspace.
  - `git grep "chrono::" | wc -l` produces `902` direct type and method invocations.
  - `git grep -E "^use (.*::)?chrono" | wc -l` produces `197` import statements across crates.
- **Declared Dependencies**:
  - `Cargo.toml:250`: `chrono = { version = "0.4", features = ["serde"] }`
  - `Cargo.toml:255`: `rusqlite = { version = "0.32.0", features = [..., "chrono", ...] }`
  - `Cargo.toml:260`: `sqlx = { version = "0.8", default-features = false, features = [..., "chrono", ...] }`
  - `crates/xavier-core-logic/Cargo.toml:12`: `chrono = { version = "0.4", features = ["serde"] }`
  - `crates/xavier-crypto/Cargo.toml:17`: `chrono = { version = "0.4", features = ["serde"] }`
  - `crates/xavier-wasm/Cargo.toml:18`: `chrono = { version = "0.4", features = ["serde"] }`
  - `code-graph/Cargo.toml:65`: `chrono = { version = "0.4", default-features = false, features = ["serde", "clock"] }`
- **Core Hotspots**:
  - `src/retrieval/gating.rs`: 22 references
  - `src/memory/sqlite_vec_store/store_impl.rs`: 19 references
  - `src/memory/sqlite_vec_store/schema_impl.rs`: 19 references
  - `src/retrieval/navigation.rs`: 18 references
  - `src/server/v1_api.rs`: 16 references
  - `crates/xavier-core-logic/src/scoring.rs`: 14 references

---

## Desired State (DELTA)
Replace `chrono` across all workspace crates with BurntSushi's `jiff` (and `jiff-sqlx` / `rusqlite/jiff` features):
- **Dependencies**:
  - Replace `chrono = "0.4"` in root `Cargo.toml` with `jiff = { version = "0.1", features = ["serde"] }`.
  - In root `Cargo.toml`, update `rusqlite` features: replace `"chrono"` with `"jiff"`.
  - In root `Cargo.toml`, add `jiff-sqlx = "0.1"` or adapt SQLx queries using `jiff::Timestamp`.
  - Replace `chrono` in `crates/xavier-core-logic`, `crates/xavier-crypto`, `crates/xavier-wasm`, and `code-graph`.
- **Type Replacements**:
  - `chrono::DateTime<chrono::Utc>` -> `jiff::Timestamp` (or `jiff::Zoned` where time zones are explicitly preserved).
  - `chrono::Utc::now()` -> `jiff::Timestamp::now()`.
  - `chrono::Utc::now().timestamp()` -> `jiff::Timestamp::now().as_second()`.
  - `chrono::Duration::hours(n)` / `days(n)` -> `jiff::Span::new().hours(n)` or `jiff::SignedDuration::from_secs(n * 3600)`.
  - `dt.to_rfc3339()` -> `ts.to_string()` (produces RFC 3339 / ISO 8601 string representation).
  - `chrono::DateTime::parse_from_rfc3339(str)` -> `str.parse::<jiff::Timestamp>()`.
- **Database Adapters**:
  - Ensure SQLite storage layers (`src/memory/sqlite_vec_store/`, `src/auth2/db.rs`) read/write timestamps using `rusqlite`'s native `jiff` feature or Unix epoch seconds without precision loss.

---

## 🌐 Web Research Required
**MANDATORY — 5 Domain-Specific Queries (Run before modifying code):**
1. `jiff rust migration from chrono best practices 2026` — Consult syntax diffs for `DateTime<Utc>` vs `jiff::Timestamp` and `Span`.
2. `rusqlite jiff feature flag ToSql FromSql` — Verify exact feature flag syntax and conversion traits for SQLite columns.
3. `jiff-sqlx postgres timestamp decode encode` — Check how `jiff-sqlx` integrates with `sqlx::Query` and custom row mapping.
4. `RUSTSEC-2026-0543 chrono unmaintained advisory` — Read the official RUSTSEC advisory and migration guidelines.
5. `jiff wasm32-unknown-unknown clock support` — Verify clock provider requirements when compiling `crates/xavier-wasm`.

---

## 🔬 Agent Session Prompt
"Before writing code, please execute the following micro-phases strictly:

1. **Phase 1: Dependencies & Configuration (Root & Crates)**
   - Add `jiff = { version = "0.1", features = ["serde"] }` to `Cargo.toml` and workspace subcrates.
   - Switch `rusqlite` feature flag from `"chrono"` to `"jiff"`.
   - Run `cargo check --features ci-safe` to verify that `jiff` resolves cleanly.

2. **Phase 2: Sub-Crates Migration (Surgical & Isolated)**
   - Migrate `crates/xavier-crypto/` (replace `chrono` with `jiff`). Verify with `cargo check -p xavier-crypto`.
   - Migrate `crates/xavier-core-logic/` (replace `chrono` with `jiff`). Verify with `cargo test -p xavier-core-logic`.
   - Migrate `crates/xavier-wasm/` and `code-graph/`. Verify compilation.

3. **Phase 3: Xavier Server, Memory & Adapters**
   - Replace imports `use chrono::{DateTime, Utc, Duration};` with `use jiff::{Timestamp, Span, SignedDuration};`.
   - Migrate `src/memory/`, `src/retrieval/`, `src/api/`, `src/auth2/`, `src/mesh/`, and `src/server/`.
   - Ensure RFC 3339 parsing and serialization remain byte-compatible for REST/MCP APIs.

4. **Phase 4: Deprecate `chrono` & Full Workspace Verification**
   - Remove `chrono` entries from all `Cargo.toml` files.
   - Run verification suite: `cargo clippy --package xavier --all-targets --features ci-safe -- -D warnings`.
   - Run unit & integration tests: `cargo test --package xavier --lib --features ci-safe -- --test-threads=1`.
   - Verify zero unmaintained advisories: `cargo audit`.

5. **Blocker Recovery**:
   - If a third-party dependency still pulls `chrono` transitively, verify that our first-party code does not invoke it directly (`git grep "chrono::" src/` must return 0).
   - If `sqlx` compilation fails on custom types, use `jiff-sqlx` wrapper types or store ISO-8601 strings / i64 unix timestamps in the database layer. NEVER stall waiting for input."

---

## Existing Code Patterns (DEBES seguir estos)

### 1. Timestamp Instantiation & Seconds Extraction
```rust
// Current Pattern (chrono)
let now_ts = chrono::Utc::now().timestamp();
let iso_str = chrono::Utc::now().to_rfc3339();

// Target Pattern (jiff)
let now_ts = jiff::Timestamp::now().as_second();
let iso_str = jiff::Timestamp::now().to_string();
```

### 2. Expiration & Relative Time Calculations
```rust
// Current Pattern (chrono)
let expires_at = chrono::Utc::now() + chrono::Duration::hours(24);

// Target Pattern (jiff)
let expires_at = jiff::Timestamp::now()
    .checked_add(jiff::Span::new().hours(24))
    .expect("timestamp overflow");
```

### 3. Parsing RFC 3339 Strings
```rust
// Current Pattern (chrono)
let dt = chrono::DateTime::parse_from_rfc3339(raw_str)?.with_timezone(&chrono::Utc);

// Target Pattern (jiff)
let ts: jiff::Timestamp = raw_str.parse()?;
```

---

## Acceptance Criteria (VERIFIABLE BY COMMAND)

### A. Compilation & Lint (Always)
- [ ] `cargo check --package xavier --all-targets --features ci-safe` — 0 errors
- [ ] `cargo clippy --package xavier --all-targets --features ci-safe -- -D warnings` — 0 warnings
- [ ] `cargo fmt --all -- --check` — clean, 0 formatting diffs

### B. Tests & Regressions (Always)
- [ ] `cargo test --package xavier --lib --features ci-safe -- --test-threads=1` — all passing, 0 failed
- [ ] `cargo test --package xavier-core-logic` — all passing, 0 failed
- [ ] `cargo test --package xavier-crypto` — all passing, 0 failed

### C. Delivery Anti-Empty (Always)
- [ ] `git status --porcelain` is clean after commit; `git diff --stat HEAD^` is not empty
- [ ] `git show HEAD --name-only | grep -cE "src/|crates/|Cargo.toml"` >= 5 (real modified source files)
- [ ] `wc -l Cargo.toml` shows modified dependencies

### D. Security & Secrets (Always)
- [ ] `grep -rniE "sk-|secret|password|token\s*=\s*['\"][^'\"]" src/ | wc -l` == 0
- [ ] No `.env` or temporary files committed
- [ ] `cargo audit` reports 0 active security warnings related to `chrono`

### E. Scope & Anti-Regression (Always)
- [ ] `.gitcore/features.json` is UNTOUCHED (reconciled only at wave close)
- [ ] No functional regression in REST API endpoints or MCP memory tools

### F. Task-Specific Rails (MOLD the Delivery)
- [ ] `git grep "chrono::" src/ crates/ code-graph/ | wc -l` == 0 (zero direct chrono calls remain)
- [ ] `git grep -w 'chrono = ' Cargo.toml crates/*/Cargo.toml code-graph/Cargo.toml | wc -l` == 0 (chrono removed as direct dependency)
- [ ] `git grep "jiff::" src/ | wc -l` >= 50 (jiff is actively adopted across core systems)
- [ ] `grep -c '"jiff"' Cargo.toml` >= 1 (`rusqlite` feature flag updated)

---

## Files to Modify
| File | Current State | Change | Risk |
|---|---|---|---|
| `Cargo.toml` | Declares `chrono` and `rusqlite = [..., "chrono"]` | Replace with `jiff`, enable `rusqlite = [..., "jiff"]` | MEDIUM |
| `crates/xavier-core-logic/Cargo.toml` | Declares `chrono = "0.4"` | Replace with `jiff = "0.1"` | LOW |
| `crates/xavier-crypto/Cargo.toml` | Declares `chrono = "0.4"` | Replace with `jiff = "0.1"` | LOW |
| `crates/xavier-wasm/Cargo.toml` | Declares `chrono = "0.4"` | Replace with `jiff = "0.1"` | LOW |
| `code-graph/Cargo.toml` | Declares `chrono = "0.4"` | Replace with `jiff = "0.1"` | LOW |
| `src/memory/sqlite_vec_store/store_impl.rs` | 19 calls to `chrono::Utc` | Migrate to `jiff::Timestamp` | HIGH |
| `src/memory/sqlite_vec_store/schema_impl.rs` | 19 calls to `chrono::Utc` | Migrate to `jiff::Timestamp` | HIGH |
| `src/retrieval/gating.rs` | 22 calls to `chrono` | Migrate to `jiff` time calculations | MEDIUM |
| `src/retrieval/navigation.rs` | 18 calls to `chrono` | Migrate to `jiff` time calculations | MEDIUM |
| `src/server/v1_api.rs` | 16 calls to `chrono` | Migrate REST request/response timestamps | MEDIUM |
| `src/api/` & `src/mesh/` & `src/auth2/` | Widespread calls to `chrono::Utc::now()` | Migrate to `jiff::Timestamp::now()` | MEDIUM |

---

## DO NOT touch (Anti-Regression)
- `.gitcore/features.json` — reconciled strictly at wave close
- Vector similarity math in `crates/xavier-core-logic/src/vector.rs`
- Node identity Ed25519 BIP39-24 keys in `src/node_identity/`
- Hardware acceleration / GGML logic in `src/inference/`

---

## Anti-Hallucination Guard ⚠️
1. **Never drop timestamp columns**: Do not delete timestamp fields from SQLite tables or JSON models to bypass compiler errors; convert them to `jiff::Timestamp`.
2. **Never hardcode string dates**: Use `jiff::Timestamp::now().to_string()` for ISO 8601 formatting, never manual string concatenation.
3. **Preserve Unix Seconds semantics**: Where functions return an `i64` unix timestamp, use `.as_second()`, NOT milliseconds or nanoseconds unless the existing field explicitly requires `_ms` / `_ns`.
4. **Zero warnings policy**: Run `cargo clippy --all-targets --features ci-safe -- -D warnings` after each subcrate migration. Do not ignore deprecations or unused imports.

---

## Verification
```bash
# 1. Verify zero direct chrono references in source code
test $(git grep "chrono::" src/ crates/ code-graph/ | wc -l) -eq 0

# 2. Verify compilation & clippy
cargo check --package xavier --all-targets --features ci-safe
cargo clippy --package xavier --all-targets --features ci-safe -- -D warnings
cargo fmt --all -- --check

# 3. Verify core test suite execution
cargo test --package xavier --lib --features ci-safe -- --test-threads=1
cargo test --package xavier-core-logic
```

---

## Dependencies & Merge Order
- **Depends on:** None (standalone technical debt & security refactor).
- **Blocked by:** None.
- **Parallel with:** Wave tasks that do not alter database migration schemas.
- **Merge order within wave:** 1/1 (self-contained tech debt epic or phased PRs).
- **Expected effort:** Large (4-6h due to 900+ call site migrations across 5 crates).

---

## Failure Recovery
| If this happens | Action |
|---|---|
| `rusqlite` fails to bind `jiff::Timestamp` | Verify that the `"jiff"` feature flag is active in `Cargo.toml` for `rusqlite`. If binding custom types fails, convert via `.to_string()` (RFC 3339) or `.as_second()` (i64). |
| `sqlx` type mapping fails | Add `jiff-sqlx` or decode database timestamps into `i64` before wrapping into `jiff::Timestamp::from_second(...)`. |
| `jiff::Span` arithmetic overflows | Use `.checked_add(...)` and return a domain `Error::InvalidTimestamp` instead of unwrapping blindly. |
| Test failures in date diffs | Ensure timezone parity: `jiff::Timestamp` is strictly UTC; do not mix with system local time without explicit `Zoned` conversion. |

---

## PR Delivery Requirements (ANTI-EMPTY-PR)
- [ ] `git status --porcelain` shows all modified files before opening PR
- [ ] `git diff --stat HEAD` is non-empty and reflects source code modifications
- [ ] The PR MUST contain >= 5 modified `.rs` files and updated `Cargo.toml`
- [ ] If work is blocked by an unresolvable compiler trait issue: DO NOT open an empty or broken PR; document the exact compiler error and blocker in the issue comments.
