# dwrs

**dwrs** is a parallel file downloader with localization support, progress bars, and colorful output — written in Rust.
It’s a fast, user-friendly alternative to tools like `wget`, designed for modern terminal workflows.

![GitHub release](https://img.shields.io/github/v/release/bircoder432/dwrs?style=flat-square)
![crates.io](https://img.shields.io/crates/v/dwrs?style=flat-square)
![License](https://img.shields.io/crates/l/dwrs?style=flat-square)

---

## ✨ Features

- 🚀 Parallel downloads (`--workers`)
- 📄 Supports download lists from file
- 🌐 Localized interface (English & Russian)
- 📦 Colorful terminal output and progress bars
- 🔄 `--continue` flag for resuming interrupted downloads
- 🔧 Lightweight and fast, built in pure Rust

---

## 📦 Installation

### Install via Cargo (recommended)

```bash
cargo install dwrs
````

Requires [Rust](https://rustup.rs) and Cargo.

### Build from source

```bash
git clone https://github.com/bircoder432/dwrs.git
cd dwrs
cargo build --release
```

Binary will be in `target/release/dwrs`.

---

## 🧑‍💻 Usage

Download a file:

```bash
dwrs --url https://example.com/file.iso
```

Specify custom output name:

```bash
dwrs --url https://example.com/file.iso --output my_file.iso
```

Download multiple files in parallel:

```bash
dwrs --url link1 link2 link3 --output out1 out2 out3 --jobs 3
```

Batch download from file (`urls.txt`):

```
https://example.com/image1.jpg img1.jpg
https://example.com/image2.jpg
```

```bash
dwrs --file urls.txt
```

Resume an interrupted download:

```bash
dwrs --url https://example.com/large_file.zip --continue
```

---

## 🌍 Localization

`dwrs` detects your system language and displays messages accordingly.

Supported languages:

* English (`en`)
* Russian (`ru`)

Localization is powered by [`rust-i18n`](https://github.com/longbridgeapp/rust-i18n).

---

## 🤝 Contributing

Contributions, feedback, and feature suggestions are welcome!
Feel free to open issues or submit pull requests.

---
