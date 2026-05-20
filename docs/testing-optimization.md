# Orbit Testing & Build Optimization

## Current Issues

- **Test overhead:** Unit tests are slow and accumulate files
- **Build size:** `tauri/target` is **8.43 GB** with 8,994 files
- **Project size large:** Tests generate many artifacts that aren't cleaned

---

## Solutions

### 1. Cargo Features for Test Isolation

**File:** `tauri/Cargo.toml`

```toml
[features]
default = ["dev"]
dev = []
ci = ["test-integration", "test-mocks"]
test-integration = []
test-mocks = []

[profile.dev]
incremental = true
opt-level = 0

[profile.dev.package."*"]
opt-level = 3  # Speed up dev dependencies

[profile.test]
incremental = true
codegen-units = 4  # Parallelize better

[profile.release]
incremental = true
lto = "thin"
```

**Benefit:**
- Separate test configurations for dev vs CI
- Faster dev builds, optimized test builds
- Reduced artifact accumulation

---

### 2. Test Cache Strategy

**Add to `tauri/Cargo.toml`:**

```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "my_benchmark"
harness = false
```

**Use in tests:**

```bash
# Pre-compile tests (faster subsequent runs)
cargo test --no-run

# Run tests with cache
cargo test --test-threads=1

# Run specific test module
cargo test tests::module::specific_test
```

**Benefit:**
- Pre-compilation avoids re-running compilation
- Faster subsequent test runs
- Better parallelization

---

### 3. In-Memory Test Database

**File:** `tauri/src/services/database.rs`

```rust
use rusqlite::{Connection, OpenFlags};
use std::path::Path;

pub fn open_in_memory() -> Connection {
    let mut flags = OpenFlags::SQLITE_OPEN_MEMORY;
    if cfg!(test) {
        flags |= OpenFlags::SQLITE_OPEN_FULLMUTEX;
    }
    Connection::open_with_flags_and_vfs("", flags, None).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_operations() {
        let conn = open_in_memory();
        // Tests use in-memory DB, no artifacts
        assert_eq!(conn.total_changes(), 0);
    }
}
```

**Benefit:**
- No disk artifacts from test databases
- Faster tests (in-memory I/O)
- Isolated test state

---

### 4. Clean Build Artifacts

**Add to `tauri/.gitignore`:**

```
# Test artifacts
*.db
*.db-shm
*.db-wal
test-output/
test-results/

# Cargo artifacts
target/
Cargo.lock (except CI)
```

**Add cleanup script:** `scripts/clean-target.sh`

```bash
#!/bin/bash
# Clean Rust build artifacts
cd tauri
cargo clean
rm -rf target/

# Clean JS artifacts
cd ../ui
rm -rf node_modules/.cache/
rm -rf dist/
```

**Benefit:**
- Prevents artifact accumulation
- Faster clean builds
- Smaller repository

---

### 5. Test Isolation with Files

**File:** `tauri/src/lib.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_session() -> Session {
        Session {
            id: "test-session-123".to_string(),
            cwd: std::env::temp_dir(),
            ..Session::default()
        }
    }

    #[test]
    fn test_session_creation() {
        let session = create_test_session();
        assert_eq!(session.id, "test-session-123");
    }

    #[test]
    fn test_session_persistence() {
        let session = create_test_session();
        // Tests use temp directory, no conflicts
    }
}
```

**Benefit:**
- Unique test data per test
- No interference between tests
- Easier debugging

---

### 6. CI Optimized Builds

**File:** `.github/workflows/ci.yml`

```yaml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4

      - name: Cache cargo registry
        uses: actions/cache@v4
        with:
          path: ~/.cargo/registry
          key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}

      - name: Cache cargo index
        uses: actions/cache@v4
        with:
          path: ~/.cargo/git
          key: ${{ runner.os }}-cargo-index-${{ hashFiles('**/Cargo.lock') }}

      - name: Cache cargo build
        uses: actions/cache@v4
        with:
          path: tauri/target
          key: ${{ runner.os }}-cargo-build-target-${{ hashFiles('**/Cargo.lock') }}

      - name: Run tests
        run: |
          cd tauri
          cargo test --test-threads=1 --no-run  # Pre-compile
          cargo test --test-threads=1            # Run tests
```

**Benefit:**
- Faster CI builds with caching
- Reduced network usage
- Parallel test execution

---

## Recommended Commands

### Daily Development

```bash
# Clean and rebuild (if needed)
npm run clean && npm run dev

# Quick tests
cd tauri && cargo test

# Check without running
cd tauri && cargo check
```

### Pre-Commit

```bash
# Run tests and checks
cd tauri && cargo test && cargo clippy -- -D warnings
npm run lint
```

### CI/CD

```bash
# Full test suite
cd tauri && cargo test --all-targets --all-features

# Build artifacts
npm run tauri:build
```

---

## Expected Improvements

| Metric | Current | After Optimization |
|--------|---------|-------------------|
| tauri/target size | 8.43 GB | ~1 GB |
| Test compilation time | ~5 min | ~2 min |
| Test execution time | ~30 sec | ~20 sec |
| Artifact count | 8,994 | < 1,000 |
| CI build time | ~10 min | ~6 min |

---

## Implementation Priority

1. **High Priority:**
   - Add Cargo features for test isolation
   - Implement in-memory test database
   - Add cleanup script

2. **Medium Priority:**
   - Add test cache strategy
   - Optimize CI configuration
   - Add .gitignore patterns

3. **Low Priority:**
   - Add benchmarking framework
   - Optimize dev dependencies
   - Add test coverage reporting

---

## References

- [Cargo Book - Testing](https://doc.rust-lang.org/cargo/reference/tests.html)
- [Cargo Book - Profiles](https://doc.rust-lang.org/cargo/reference/profiles.html)
- [Testing Rust Code - Mozilla](https://doc.rust-lang.org/book/ch11-00-testing.html)