# dwrs

A CLI file downloader with parallel downloading, resume support, and batch mode.

---

## Installation

### Via Cargo

```bash 
cargo install dwrs
```

### Build from source

```bash 
git clone https://github.com/bircoder432/dwrs.git
cd dwrs
cargo build --release
```

Binary:

```text 
target/release/dwrs
```

---

## Usage

### Download a file

```bash 
dwrs https://example.com/file.iso
```

Multiple files:

```bash 
dwrs url1 url2 url3
```

---

### Output files (IMPORTANT: repeated flag)

Output is specified per file using repeated `-o` flags:

```bash 
dwrs url1 url2 -o out1 -o out2
```

Each `-o` corresponds to the URL at the same position.

If outputs are omitted, filenames are derived from URLs.

---

### Download from file

Format:

```text 
https://example.com/file1.zip output1.zip
https://example.com/file2.zip
# comments supported
```

Run:

```bash 
dwrs --file urls.txt
```

---

### Resume download

```bash 
dwrs https://example.com/file.zip --continue
```

---

### Background mode

```bash 
dwrs https://example.com/file.iso --background
```

---

## Options

```text 
-w, --workers <N>              Parallel chunks per file (default: 4)
-o, --output <FILE>           Output filename (repeatable per URL)
-r, --retries <N>             Retry attempts (default: 3)
--buffer-size <KB>            Write buffer size in KB
--pool-size <N>               HTTP connection pool size (default: 100)
--max-files <N>               Max concurrent file downloads
--min-parallel-size <MB>     Minimum size for parallel download
-c, --continue               Resume interrupted downloads
-f, --file <PATH>            Input file with URL list
--config <PATH>              Custom config file
--background                 Run in background
```

---

## Configuration

Default path:

```text 
~/.config/dwrs/config.toml
```

### Example

```toml 
workers = 8
buffer_size = 262144
pool_size = 100
retries = 3
min_parallel_size = 5242880

msg_template = "→ {output}"
template = "{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} ({percent}%) {msg}"
bar_chars = "█▌░"
```

---

## Behavior

### Parallel download

* Uses HTTP Range requests
* Splits into chunks
* Downloads concurrently
* Merges final file
* Falls back to single-thread if unsupported

### Resume

* Works for both sequential and parallel modes
* Requires server support for Range

---

## Notes

* `-o` is positional by repetition, not a list
* URL → output mapping is index-based
* Parallel mode depends on server `Accept-Ranges`

---

## License

MIT
