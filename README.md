# AQJ Package Manager 📦

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Language: Rust](https://img.shields.io/badge/Language-Rust-orange.svg)](https://www.rust-lang.org/)

**AQJ** adalah sistem manajemen paket modular, efisien, dan cepat yang terinspirasi oleh XBPS, dikembangkan dengan bahasa pemrogaman Rust. System ini didesain untuk kompilasi paket dari resep (recipe-based source build engine), manajemen database lokal, instalasi, dan penghapusan paket secara terstruktur.

---

## 🚀 Fitur Utama

- **Arsitektur Modular**: Terbagi menjadi crate terpisah (`core`, `src`, `install`, `remove`, `query`, dan `cli` wrapper).
- **Format Resep TOML**: Penulisan instruksi build paket yang deklaratif dan mudah dibaca menggunakan `template.toml`.
- **Pengarsipan & Kompresi Cepat**: Mendukung format arsip tarball yang dikompresi dengan `zstd` / `gzip`.
- **Database Lokal Terstruktur**: Pelacakan metadata paket yang terinstall beserta checksum SHA-256 daftar file.
- **Antarmuka CLI Terpadu**: CLI utama (`aqj`) yang membungkus sub-perintah manajemen paket.

---

## 🧱 Struktur Workspace (Crates)

Project ini disusun sebagai sebuah Rust workspace yang terdiri dari beberapa sub-crate:

| Crate | Deskripsi |
|---|---|
| `crates/aqj-core` | Library inti yang menangani database JSON, pengarsipan, metadata, dan hashing SHA-256. |
| `crates/aqj-cli` | Pembungkus CLI utama (`aqj`) untuk memanggil fungsi installer, builder, remover, dan query. |
| `crates/aqj-src` | Engine pembuat paket yang membaca resep `template.toml`, mengunduh source, mengompilasi, dan membuat file arsip `.aqj`. |
| `crates/aqj-install` | Utility untuk meng-ekstrak dan mendaftarkan arsip `.aqj` ke database sistem. |
| `crates/aqj-remove` | Utility untuk menghapus file yang terinstall dan memperbarui database. |
| `crates/aqj-query` | Utility untuk menampilkan daftar dan informasi detail paket yang terinstall. |

---

## 💻 Cara Penggunaan

### 1. Kompilasi & Build Project

Pastikan Anda telah menginstall toolchain Rust (`cargo` & `rustc`).

```bash
# Clone repository
git clone https://github.com/Suyono-Sukorame/aqj.git
cd aqj

# Build seluruh workspace dalam mode release
cargo build --release
```

Hasil binary akan berada di folder `target/release/aqj`.

---

### 2. Perintah CLI `aqj`

#### A. Membuat Paket dari Resep (`aqj src`)
Membaca file `template.toml` dari folder resep dan memproses kompilasi hingga menjadi paket `.aqj`:

```bash
cargo run --release -p aqj-cli -- src build aqj-packages/pkgs/hello
```

#### B. Menginstall Paket (`aqj install`)
Menginstall file binary/arsip `.aqj` ke dalam sistem:

```bash
cargo run --release -p aqj-cli -- install hello-2.12.1_1.x86_64.aqj
```

#### C. Kueri / Mengecek Paket Terinstall (`aqj query`)
Melihat daftar paket yang terpasang di database sistem:

```bash
cargo run --release -p aqj-cli -- query -l
```

Melihat detail paket tertentu:

```bash
cargo run --release -p aqj-cli -- query -i hello
```

#### D. Menghapus Paket (`aqj remove`)
Menghapus paket beserta file-file yang didaftarkannya dari sistem:

```bash
cargo run --release -p aqj-cli -- remove hello
```

---

## 📝 Contoh Resep Paket (`template.toml`)

Berikut adalah contoh resep pembuatan paket `hello`:

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

## 📄 Lisensi

Project ini dilesensikan di bawah lisensi **GNU General Public License v3.0** (`GPL-3.0-or-later`).
Lihat berkas [LICENSE](LICENSE) untuk informasi lebih rincinya.
