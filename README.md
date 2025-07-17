# Portable Network Archive (PNA)

[![Crates.io](https://img.shields.io/crates/v/portable-network-archive.svg)](https://crates.io/crates/portable-network-archive)
[![Downloads](https://img.shields.io/crates/d/portable-network-archive.svg)](https://crates.io/crates/portable-network-archive)
[![Docs.rs](https://docs.rs/portable-network-archive/badge.svg)](https://docs.rs/portable-network-archive)
[![Test](https://github.com/ChanTsune/Portable-Network-Archive/actions/workflows/test.yml/badge.svg)](https://github.com/ChanTsune/Portable-Network-Archive/actions/workflows/test.yml)
![License](https://img.shields.io/crates/l/portable-network-archive.svg)

<div align="center">
  <img src="./icon.svg" alt="PNA" width="100"/>
</div>

**Portable Network Archive (PNA)** is a flexible, secure, and cross-platform archive format inspired by the PNG data structure. It combines the simplicity of ZIP with the robustness of TAR, providing efficient compression, strong encryption, and seamless splitting and streaming capabilities.

### Why PNA?

**Portable Network Archive (PNA): A Flexible, Secure, and Cross-Platform Archive Format**
- **Portability:** Works seamlessly across multiple platforms, combining the strengths of TAR and ZIP formats.
- **Compression Flexibility:** Advanced per-file and archive-wide compression options reduce the need for full archive decompression.
- **Encryption & Security:** Supports 256-bit AES and Camellia for robust protection of sensitive data.
- **Splittable Structure**: Based on PNG’s data unit structure, enabling the easy division of large archives into smaller parts.
- **Streamability:** Supports serial read and write operations, making it suitable for streaming processing, similar to a TAR format.
- **Extensibility**: Designed to accommodate future extensions and private add-ons, ensuring compatibility with the basic PNA format while allowing for flexible customization.
- **Error Resilience:** File integrity checks and error detection ensure data is secure during transmission.

Additionally, the PNA specification includes a rationale appendix to help developers understand key design choices, making implementation more straightforward.

## Features

- **File Compression and Decompression**
  - [x] Supports zlib, zstd, and xz.

- **File Encryption and Decryption**
  - [x] Supports 256-bit AES and 256-bit Camellia.

- **Solid Mode**
  - [x] Compresses and encrypts the entire archive as a single block.

- **File Attribute Preservation (Maintains and restores)**
  - [x] File permissions.
  - [x] File timestamps.
  - [x] Extended attributes.
  - [x] Access Control Lists (ACLs) (experimental).

## CLI Supported Platform
- Cross-platform support including Windows, Linux, macOS, and FreeBSD  
  _(Support for additional platforms planned.)_

## Installation

### Via Shell (Prebuilt Binary)

#### On Linux or MacOS

```sh
curl --proto '=https' --tlsv1.2 -LsSf 'https://github.com/ChanTsune/Portable-Network-Archive/releases/latest/download/portable-network-archive-installer.sh' | sh
```

#### On Windows

```sh
powershell -ExecutionPolicy Bypass -c "irm https://github.com/ChanTsune/Portable-Network-Archive/releases/latest/download/portable-network-archive-installer.ps1 | iex"
```

### Via Cargo

```sh
cargo install portable-network-archive
```

### From Source (via Cargo)

```sh
cargo install --git https://github.com/ChanTsune/Portable-Network-Archive.git portable-network-archive
```

## Basic Usage

Create an archive:
```sh
pna create <ARCHIVE> [FILES]...
```

Extract an archive:
```sh
pna extract <ARCHIVE>
```

List archive contents:
```sh
pna list <ARCHIVE>
```

For more commands and options:
```sh
pna --help
```

### Subcommands Overview

| Command | Description |
|---------|-------------|
| `create` | Create archive (alias: `c`) |
| `append` | Append files to an archive (alias: `a`) |
| `extract` | Extract files from an archive (alias: `x`) |
| `list` | List files in an archive (aliases: `l`, `ls`) |
| `split` | Split archive into parts |
| `concat` | Concatenate split archives |
| `strip` | Strip metadata from archive entries |
| `xattr` | Manipulate extended attributes |
| `complete` | Generate shell completion scripts |
| `bug-report` | Generate bug report template |
| `experimental` | Experimental unstable commands |

### Example Session

```console
$ pna create archive.pna myfile.txt
Create an archive: archive.pna
Successfully created an archive in 0 seconds

$ pna list archive.pna
myfile.txt

$ pna extract archive.pna -C output_dir
Extract archive archive.pna
Successfully extracted an archive in 0 seconds
```

## Specification

Detailed information is available in the [Specification](https://portable-network-archive.github.io/Portable-Network-Archive-Specification/) document.

# License

This project is licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](./LICENSE-APACHE) or
http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](./LICENSE-MIT) or
http://opensource.org/licenses/MIT)

at your option.

### Contribution

Contributions are welcome! Please open an issue or pull request on GitHub.
Before submitting, make sure to run:

```sh
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
