# QR Transfer

<p align="center">
  <strong>Decentralized Encrypted File Transfer via Animated QR Codes</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Protocol-v0.1-blue" alt="Protocol Version">
  <img src="https://img.shields.io/badge/License-MIT%2FApache--2.0-green" alt="License">
  <img src="https://img.shields.io/badge/Rust-1.78%2B-orange" alt="Rust Version">
  <img src="https://img.shields.io/badge/Web-PWA%20%7C%20WASM-purple" alt="Web Platform">
</p>

---

## Vision

**Transfer any file between any two devices without servers, accounts, networks, or trust.**

QR Transfer is a decentralized, end-to-end encrypted file transfer system that uses animated QR codes as its exclusive data channel. It requires no internet connection, no Bluetooth pairing, no cloud storage, and no pre-shared secrets. Any device with a screen and any device with a camera can exchange files securely.

---

## Features

### Security
- **X25519 ECDH** key exchange with 128-bit security
- **Ed25519** identity key signing for authentication
- **AES-256-GCM** authenticated encryption per chunk
- **Safety Number** verification for MITM detection (Signal-style)
- **Session key rotation** every 15 minutes or 10,000 chunks
- **BLAKE3** file integrity verification
- **Deterministic fountain seeding** from shared secret

### Reliability
- **Fountain codes** (RaptorQ/LT) for lossy optical channels
- **Resume/pause** support for interrupted transfers
- **Multi-QR grid** display for 2-9x throughput multiplier
- **Adaptive FPS** based on decode success rate
- **CRC-32** per-frame integrity checking

### Cross-Platform
- **Web PWA** — Single HTML file, works in any browser
- **CLI Tool** — Rust-based command-line reference implementation
- **Native SDKs** — iOS (Swift) and Android (Kotlin) via Rust FFI

### Accessibility
- **High-contrast QR mode** (21:1 contrast ratio)
- **Reduced motion** support via `prefers-reduced-motion`
- **Screen reader** compatible with ARIA live regions
- **WCAG 2.1 AA** compliant

---

## Quick Start

### Web PWA (Browser)

Open `platforms/web/dist/index.html` in any modern browser:

```bash
cd platforms/web
npm install
npm run build
npm run preview
```

Or use the hosted version at [qrtransfer.dev](https://qrtransfer.dev)

### CLI Tool

```bash
# Build the Rust workspace
cargo build --release

# Send a file
./target/release/qr-transfer send document.pdf

# Receive a file
./target/release/qr-transfer receive --output ./downloads/

# Generate identity keypair
./target/release/qr-transfer keygen --export ./identity.pem

# Benchmark performance
./target/release/qr-transfer benchmark
```

---

## Architecture

```
qr-transfer/
├── crates/
│   ├── qr-crypto/        # X25519, AES-GCM, HKDF, BLAKE3, key rotation
│   ├── qr-protocol/      # Frame format, state machine, capability handshake
│   ├── qr-codec/         # QR generation, grid packing, interleave patterns
│   ├── qr-fountain/      # RaptorQ/LT fountain codes
│   ├── qr-compression/   # zstd compression with file type detection
│   └── qr-cli/           # Command-line reference implementation
├── platforms/
│   └── web/              # TypeScript + Vite PWA
├── docs/
│   └── spec/             # Formal protocol specification
└── tools/
    ├── bench/            # Benchmark suite
    └── fuzz/             # Fuzzing harnesses
```

---

## Protocol Overview

### Frame Format

| Field           | Size   | Purpose                          |
|-----------------|--------|----------------------------------|
| Magic           | 2 B    | Protocol ID (`0x5152` = "QR")    |
| Flags           | 2 B    | Version, ECC, Auth, Grid         |
| Chunk Index     | 4 B    | Block position in sequence       |
| Total Chunks    | 4 B    | Total blocks expected            |
| File Hash Prefix| 8 B    | BLAKE3 prefix for verification   |
| Key Epoch       | 2 B    | Key rotation counter             |
| Payload         | ~1400 B| Encrypted data / fountain block  |
| CRC-32          | 4 B    | Per-frame integrity check        |

### Transfer Lifecycle

1. **Key Exchange** — X25519 ephemeral keypairs exchanged via QR
2. **Safety Number Verification** — Human-verifiable MITM check
3. **Capability Handshake** — Agree on QR version, ECC level, grid
4. **Encrypted Transfer** — AES-256-GCM + fountain encoded QR frames
5. **Integrity Verification** — BLAKE3 hash verification

---

## Performance

| Scenario              | FPS | Payload   | Throughput  | 10 MB Time |
|-----------------------|-----|-----------|-------------|------------|
| Conservative browser  | 3   | ~400 B    | ~1.2 KB/s   | ~2.3 hr    |
| Typical browser       | 5   | ~1,500 B  | ~6.4 KB/s   | ~26 min    |
| WASM optimized        | 15  | ~1,900 B  | ~48 KB/s    | ~3.5 min   |
| Native mobile         | 30  | ~2,000 B  | ~102 KB/s   | ~1.7 min   |
| High-performance      | 60  | ~2,000 B  | ~306 KB/s   | ~33 sec    |

---

## Security Analysis

| Threat                | Mitigation                                    |
|-----------------------|-----------------------------------------------|
| Eavesdropping         | AES-256-GCM encryption                        |
| Man-in-the-Middle     | Ed25519 signing + Safety Number verification  |
| Replay attacks        | Unique IV per chunk + ephemeral keys          |
| Data corruption       | Reed-Solomon ECC + CRC-32 + AES-GCM auth tag  |
| Integrity tampering   | AES-GCM auth tag + BLAKE3 file hash           |
| Side-channel attacks  | Web Crypto API constant-time implementations  |

---

## Competitive Comparison

| Feature           | QR Transfer | LocalSend | AirDrop | Signal |
|-------------------|-------------|-----------|---------|--------|
| Zero setup        | ✅          | ❌        | ❌      | ❌     |
| Fully offline     | ✅          | ✅        | ✅      | ❌     |
| Cross-platform    | ✅          | ✅        | ❌      | ✅     |
| E2E encrypted     | ✅          | ❌        | ❌      | ✅     |
| MITM resistant    | ✅          | ❌        | ❌      | ✅     |
| No accounts       | ✅          | ✅        | ❌      | ❌     |
| Speed             | 1-100 KB/s  | 10 MB/s   | 20 MB/s | 1 MB/s |

---

## Development

### Building

```bash
# Build all Rust crates
cargo build --release

# Run tests
cargo test --workspace

# Build Web PWA
cd platforms/web
npm install
npm run build

# Run benchmarks
cargo run --release -- benchmark
```

### Testing

```bash
# Unit tests
cargo test --workspace

# Integration tests
cargo test --test integration

# Physical condition tests (requires camera)
cd platforms/web
npm run test:physical
```

---

## Specification

The formal protocol specification is in `docs/spec/`:

- `01-intro.md` — Introduction and design goals
- `02-frame-format.md` — Byte-exact frame layout
- `03-capability-handshake.md` — Negotiation protocol
- `04-crypto.md` — Cryptographic profile
- `05-fountain.md` — Fountain code parameters
- `06-qr-codec.md` — QR encoding requirements
- `07-accessibility.md` — WCAG compliance
- `08-security-considerations.md` — Threat model

---

## Roadmap

| Phase | Duration | Focus                        |
|-------|----------|------------------------------|
| 0     | 4 weeks  | Spec + Security foundation   |
| 1     | 4 weeks  | Reliability core             |
| 2     | 3 weeks  | UX + Accessibility           |
| 3     | 6 weeks  | Platform SDKs (iOS/Android)  |
| 4     | 4 weeks  | Advanced features            |
| 5     | 4 weeks  | Finalization + Audit         |

---

## License

Dual-licensed under MIT and Apache-2.0. See `LICENSE-MIT` and `LICENSE-APACHE`.

---

## Contributing

See `CONTRIBUTING.md` for guidelines. All contributions must pass:

- `cargo test --workspace`
- `cargo clippy --workspace`
- `cargo fmt --check`
- axe-core accessibility audit (web)

---

## Acknowledgments

- TxQR — Pioneer of fountain code QR transfer
- ShadowCat — Browser-first minimal implementation
- qr-send — Performance frontier with WASM ZBar
- Signal — Safety Number concept and UI patterns