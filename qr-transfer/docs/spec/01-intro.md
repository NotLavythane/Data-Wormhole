# QR Transfer Protocol Specification v0.1

## 1. Introduction

QR Transfer is a decentralized, end-to-end encrypted file transfer protocol that uses animated QR codes as its exclusive data channel. This document specifies the protocol design goals, requirements, and scope.

## 2. Design Goals

1. **Universal Compatibility**: Works between any two devices with a screen and a camera
2. **Zero Infrastructure**: No servers, no accounts, no cloud, no DNS, no certificates
3. **End-to-End Encryption**: X25519 ECDH + AES-256-GCM with forward secrecy
4. **Loss Resilience**: Fountain codes ensure reliable transfer over lossy optical channels
5. **MITM Resistance**: Safety Number verification provides human-in-the-loop authentication
6. **Accessibility**: WCAG 2.1 AA compliance for all users

## 3. Requirements

### 3.1 Functional Requirements

- Transfer files of arbitrary size between devices
- Work completely offline (no network connectivity)
- Support any file type
- Resume interrupted transfers
- Verify file integrity cryptographically

### 3.2 Non-Functional Requirements

- Transfer speed: minimum 1 KB/s in browser environments
- Latency: first QR frame displayed within 2 seconds of file selection
- Memory: maximum 50 MB RAM usage on mobile browsers
- Battery: adaptive FPS to conserve power

### 3.3 Security Requirements

- 128-bit minimum security level for all cryptographic operations
- Forward secrecy: compromise of one session does not affect others
- Identity verification: users can verify who they are exchanging with
- Metadata protection: filename and file type are encrypted

## 4. Scope

### In Scope

- Point-to-point file transfer via animated QR codes
- X25519/Ed25519 key exchange and signing
- AES-256-GCM chunk encryption with key rotation
- RaptorQ fountain codes for reliability
- BLAKE3 file integrity verification
- Multi-QR grid display for throughput multiplication

### Out of Scope (Future Versions)

- WebRTC fallback for LAN transfers
- Directory / multi-file transfer
- Streaming partial file access
- Native SDK implementations (iOS/Android)

## 5. Terminology

- **Sender**: Device displaying animated QR codes
- **Receiver**: Device scanning QR codes with camera
- **Frame**: A single QR code containing protocol data
- **Block**: A fountain-encoded data unit
- **Chunk**: An encrypted file segment
- **Session**: A complete transfer from key exchange to completion