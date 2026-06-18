# Obscura Stealth Codec

![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)
![Language](https://img.shields.io/badge/Language-Rust_100%25-orange.svg)
![Status](https://img.shields.io/badge/Status-Research_Prototype-blue.svg)
![Build](https://github.com/sangaafilum/Obscura/actions/workflows/rust.yml/badge.svg)

Obscura is a micro-footprint, zero-dependency stealth cryptographic container and lossless compressor built entirely in Rust using pure mathematical algorithms. 

Designed initially as an experimental prototype for "Pulse computing" and embedded systems, Obscura provides unbreakable obfuscation by destroying byte boundaries and blending compressed data into perfect white noise.

## 🦀 Why Rust?
In the world of cryptography and embedded systems, memory safety is paramount. Obscura is written in 100% safe Rust because:
- **Zero-cost abstractions:** We get high-level AI dispatching without sacrificing the C-like bare-metal performance needed for bit-shifting.
- **No Garbage Collector:** Predictable latency and no runtime overhead, crucial for drone firmware and IoT devices.
- **Memory Safety:** Rust's ownership model guarantees no buffer overflows or use-after-free vulnerabilities, ensuring the cryptographic integrity of the container.

## 🚀 Features

* **Stealth Encryption:** No file headers. No magic bytes. No predictable structure. The output file is mathematically indistinguishable from random noise, making reverse-engineering or signature analysis virtually impossible.
* **Multi-Pass AI Dispatcher:** The compressor evaluates 8 different compression paths in RAM (including raw passthrough and Delta filtering) to guarantee mathematically optimal lossless compression for any chunk of data.
* **Hybrid Entropy Coding:** Uses proprietary combinations of Fibonacci Coding, Elias-Gamma exponential coding, and LZ77-style Hash Chains.
* **Fractal Mirror Hash (Vector V):** A custom, extremely fast PRNG cipher that uses golden ratio hashing to protect data.
* **Absolute Integrity:** Built-in Adler-32 checksums ensure the container cannot be decrypted if tampered with or if the wrong password is provided.
* **Zero Dependencies:** Compiles to a tiny binary. Runs on bare-metal. Perfect for IoT, drones, and highly constrained embedded systems.

## 🛠 Usage

Obscura is designed as a fast CLI tool.

**Compress & Encrypt:**
```bash
cargo run --release -- compress <input_file> <output_file> <password>
```

**Decompress & Decrypt:**
```bash
cargo run --release -- decompress <input_file> <output_file> <password>
```

If no password is provided, Obscura will refuse to encode data in plaintext and will automatically generate a highly secure key for you.

## 🧠 Architecture Overview

Obscura processes files in 1 MB chunks. For each chunk, it tests the following strategies in RAM:
1. `Path 0`: Raw Passthrough (prevents bloat on pre-compressed media like PNG/MP4)
2. `Path 1`: Direct Fibonacci Bit-Shift Encoding
3. `Path 2`: LZ77 Hash Chains (Phrases)
4. `Path 3`: Elias-Gamma Dictionary Encoding
5. `Paths 4-7`: The above paths pre-processed through a Delta-Encoding filter (perfect for uncompressed WAV/BMP media).

The shortest byte-vector wins, is instantly encrypted via the Fractal Mirror Hash, and written to disk.

## 🤝 Contributing
This project is open for improvements! Whether you are a cryptographer, an embedded systems engineer, or a passionate Rustacean, your PRs are welcome. 
We are especially interested in:
- Security audits of the `Fractal Mirror Hash`.
- Speed optimizations in the multi-pass AI dispatcher.
- Ports to `no_std` for bare-metal ARM microcontrollers.

Feel free to open an Issue or submit a Pull Request.

## 🔒 Security Note
This is a research prototype. While the stealth mechanisms and custom PRNG obfuscate data perfectly against traditional analysis, it has not been audited by professional cryptographers to withstand state-level attacks. Use for educational, research, or highly specialized covert communication purposes.

## License
MIT License
