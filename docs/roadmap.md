# AQJ Package Manager - Future Development Roadmap

This document outlines the architectural roadmap and proposed future enhancements for the AQJ Package Manager project.

---

## 🔍 Current Architecture Overview

- **`crates/aqj-core`**: Handles JSON database storage (`installed.json`), file conflict checks, tarball packaging/extraction (`.aqj`), SHA-256 calculation, and package metadata.
- **`crates/aqj-src`**: Reads recipe templates (`template.toml`), downloads source code, executes build scripts, and packages binary `.aqj` archives.
- **`crates/aqj-install`**, **`crates/aqj-remove`**, **`crates/aqj-query`**: Core package operation binaries.
- **`crates/aqj-cli`**: Unified CLI tool (`aqj`).

---

## 🚀 Development Roadmap & Proposed Features

### 1. Dependency Resolution Engine (Topological & SAT Solver)
* **Goal**: Enable automated transitive dependency resolution, version constraint parsing (`>=`, `<=`, `=`), cyclic dependency detection, and topological execution order sorting.
* **Impact**: Critical core feature for robust package management.

### 2. Remote Repository Sync & Network Fetcher (`aqj sync`)
* **Goal**: Create binary repository index files (`repodata.zst` or `index.json`) and implement `aqj sync` / `aqj update` to download packages directly from remote HTTP mirrors.
* **Impact**: Enables central repository distribution.

### 3. Transactional Atomic Operations & Upgrades (`aqj upgrade`)
* **Goal**: Implement multi-stage atomic transactions (`Prepare` -> `Unpack to Temp` -> `Atomic Swap` -> `Commit DB`) with full rollback capabilities if installation fails mid-process.
* **Impact**: Ensures system integrity and crash-resilient package updates.

### 4. System Triggers & Post-Install Hooks (`hooks.d`)
* **Goal**: Add support for `post-install.sh`, `pre-remove.sh`, and system triggers (e.g. `ldconfig`, font cache update, system user generation).
* **Impact**: Ensures proper OS integration upon package installation.

### 5. Package Signing & Cryptographic Verification
* **Goal**: Integrate Ed25519 or GPG digital signature signing for `.aqj` archives and index files.
* **Impact**: Prevents tampered binary distribution and supply-chain attacks.

### 6. Isolated Sandboxed Build Environment (`bwrap` / `chroot`)
* **Goal**: Isolate `aqj-src` build steps using Linux unshare namespaces or `bwrap` (Bubblewrap).
* **Impact**: Guarantees clean, reproducible builds and protects host environment.

### 7. High-Performance Database Backend (`redb` / SQLite)
* **Goal**: Migrate from single-file `installed.json` to an embedded key-value DB (`redb`) or SQLite.
* **Impact**: $O(1)$ fast file ownership queries and lock-safe concurrent transactions.

---

## 📊 Priority Matrix

| Priority | Feature | Complexity | Impact | Status |
|---|---|---|---|---|
| 1️⃣ | **Dependency Resolution Engine** | Medium | 🔥 Critical | 🚧 In Progress |
| 2️⃣ | **Remote Repository Sync (`aqj sync`)** | High | 🔥 Critical | ⏳ Planned |
| 3️⃣ | **Atomic Upgrades & Rollbacks (`aqj upgrade`)** | High | ⚡ High | ⏳ Planned |
| 4️⃣ | **Post-Install Hooks & System Triggers** | Medium | ⚡ High | ⏳ Planned |
| 5️⃣ | **Package Signing (Ed25519)** | Medium | 🔒 Security | ⏳ Planned |
| 6️⃣ | **Sandboxed Builds (`bwrap`)** | High | 🛠️ Quality | ⏳ Planned |
| 7️⃣ | **High-Performance Database (`redb`)** | Medium | 🚀 Scale | ⏳ Planned |
