# AQJ Package Manager 📦

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Language: Rust](https://img.shields.io/badge/Language-Rust-orange.svg)](https://www.rust-lang.org/)

**AQJ** is a modular, fast, and efficient package management system inspired by XBPS and built with Rust. It features a recipe-based source build engine, local database tracking, archive management, and CLI utilities for installing, querying, and removing packages.

---

## 🚀 Key Features

- **Modular Architecture**: Structured as a Rust workspace with dedicated crates (`core`, `src`, `install`, `remove`, `query`, and a unified `cli` wrapper).
- **TOML Recipe Format**: Declarative and human-readable package recipes using `template.toml`.
- **Fast Archiving & Compression**: Support for tarball archives compressed with `zstd` / `gzip`.
- **Structured Local Database**: Tracks installed packages, metadata, and file lists with SHA-256 checksums.
- **Unified CLI Interface**: Main `aqj` executable unifying package management subcommands.

---

## 🧱 Workspace Crates

This repository is organized as a Cargo workspace with the following crates:

| Crate | Description |
|---|---|
| `crates/aqj-core` | Core library handling database storage, archive creation/extraction, metadata, and SHA-256 hashing. |
| `crates/aqj-cli` | Primary CLI wrapper (`aqj`) interfacing with builder, installer, uninstaller, and query tools. |
| `crates/aqj-src` | Recipe build engine that parses `template.toml`, fetches sources, compiles, and packages binary `.aqj` archives. |
| `crates/aqj-install` | Installer utility to unpack `.aqj` package archives and register files into the local system database. |
| `crates/aqj-remove` | Uninstaller utility to safely remove installed package files and update the database. |
| `crates/aqj-query` | Query tool to list, search, and inspect details of installed packages. |

---

## 💻 Usage

### 1. Build from Source

Ensure you have the Rust toolchain (`cargo` & `rustc`) installed.

```bash
# Clone the repository
git clone https://github.com/Suyono-Sukorame/aqj.git
cd aqj

# Build all workspace crates in release mode
cargo build --release
```

The compiled binaries will be located under `target/release/`.

---

### 2. `aqj` CLI Commands

#### A. Build a Package from Recipe (`aqj src`)
Parse a recipe template directory and compile it into a binary `.aqj` package:

```bash
cargo run --release -p aqj-cli -- src build aqj-packages/pkgs/hello
```

#### B. Install a Package (`aqj install`)
Install a compiled `.aqj` package archive into the system:

```bash
cargo run --release -p aqj-cli -- install hello-2.12.1_1.x86_64.aqj
```

#### C. Query Installed Packages (`aqj query`)
List all packages currently registered in the database:

```bash
cargo run --release -p aqj-cli -- query -l
```

Inspect details for a specific package:

```bash
cargo run --release -p aqj-cli -- query -i hello
```

#### D. Remove a Package (`aqj remove`)
Remove an installed package and its associated files from the system:

```bash
cargo run --release -p aqj-cli -- remove hello
```

---

## 📝 Package Recipe Example (`template.toml`)

Here is an example package recipe for `hello`:

```toml
[package]
name = "hello"
version = "2.12.1"
revision = 1
architecture = "x86_64"
summary = "GNU Hello World program"
license = "GPL-3.0-or-later"
homepage = "https://www.gnu.org/software/hello/"
depends = []

[source]
url = "https://ftp.gnu.org/gnu/hello/hello-2.12.1.tar.gz"
sha256 = "8d99142afd92576f30b0cd7cb42a8dc6809998bc5d607d88761f512e26c7db20"

[build]
script = """
./configure --prefix=/usr
make -j$(nproc)
make DESTDIR="${DESTDIR}" install
"""
```

---

## 📄 License

This project is licensed under the **GNU General Public License v3.0** (`GPL-3.0-or-later`).
See the [LICENSE](LICENSE) file for more details.
