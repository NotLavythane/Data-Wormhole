/**
 * Web Crypto API implementation for QR Transfer
 * Provides: X25519 key exchange, AES-GCM encryption, HKDF, BLAKE3 hashing
 */

// Generate ephemeral X25519 keypair using Web Crypto API
export async function generateKeypair(): Promise<CryptoKeyPair> {
  const kp = await window.crypto.subtle.generateKey(
    { name: 'X25519' },
    true,
    ['deriveBits']
  );
  return kp as CryptoKeyPair;
}

// Export public key as raw bytes
export async function exportPublicKey(key: CryptoKey): Promise<Uint8Array> {
  return new Uint8Array(await window.crypto.subtle.exportKey('raw', key));
}

// Import raw public key
export async function importPublicKey(rawKey: Uint8Array): Promise<CryptoKey> {
  return await window.crypto.subtle.importKey(
    'raw',
    rawKey as any,
    { name: 'X25519' },
    false,
    []
  );
}

// Derive shared secret via ECDH
export async function deriveSharedSecret(
  privateKey: CryptoKey,
  publicKey: CryptoKey
): Promise<Uint8Array> {
  const bits = await window.crypto.subtle.deriveBits(
    { name: 'X25519', public: publicKey },
    privateKey,
    256
  );
  return new Uint8Array(bits);
}

// Derive AES key + IV from shared secret via HKDF
export async function deriveSessionKeys(
  sharedSecret: Uint8Array
): Promise<{ aesKey: CryptoKey; iv: Uint8Array }> {
  const baseKey = await window.crypto.subtle.importKey(
    'raw',
    sharedSecret as any,
    'HKDF',
    false,
    ['deriveKey', 'deriveBits']
  );

  const aesKey = await window.crypto.subtle.deriveKey(
    {
      name: 'HKDF',
      hash: 'SHA-256',
      salt: new Uint8Array(0),
      info: new TextEncoder().encode('qr-aes-v1-key'),
    },
    baseKey,
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt', 'decrypt']
  );

  const ivBits = await window.crypto.subtle.deriveBits(
    {
      name: 'HKDF',
      hash: 'SHA-256',
      salt: new Uint8Array(0),
      info: new TextEncoder().encode('qr-aes-v1-nonce'),
    },
    baseKey,
    96
  );

  return { aesKey, iv: new Uint8Array(ivBits) };
}

// Derive per-chunk IV
export async function deriveChunkIV(
  sessionSalt: Uint8Array,
  chunkIndex: number,
  keyEpoch: number
): Promise<Uint8Array> {
  const baseKey = await window.crypto.subtle.importKey(
    'raw',
    sessionSalt as any,
    'HKDF',
    false,
    ['deriveBits']
  );

  const info = new Uint8Array(14);
  info.set(new TextEncoder().encode('chunk'), 0);
  const view = new DataView(info.buffer, info.byteOffset, info.byteLength);
  view.setUint32(5, chunkIndex, true);
  view.setUint16(9, keyEpoch, true);

  const bits = await window.crypto.subtle.deriveBits(
    {
      name: 'HKDF',
      hash: 'SHA-256',
      salt: new Uint8Array(0),
      info,
    },
    baseKey,
    96
  );

  return new Uint8Array(bits);
}

// Encrypt chunk with AES-256-GCM
export async function encryptChunk(
  key: CryptoKey,
  iv: Uint8Array,
  plaintext: Uint8Array
): Promise<Uint8Array> {
  const ciphertext = await window.crypto.subtle.encrypt(
    { name: 'AES-GCM', iv: iv as any },
    key,
    plaintext as any
  );
  return new Uint8Array(ciphertext);
}

// Decrypt chunk with AES-256-GCM
export async function decryptChunk(
  key: CryptoKey,
  iv: Uint8Array,
  ciphertext: Uint8Array
): Promise<Uint8Array> {
  const plaintext = await window.crypto.subtle.decrypt(
    { name: 'AES-GCM', iv: iv as any },
    key,
    ciphertext as any
  );
  return new Uint8Array(plaintext);
}

// Compute file hash (SHA-256 for browser)
export async function hashFile(data: Uint8Array): Promise<Uint8Array> {
  const hash = await window.crypto.subtle.digest('SHA-256', data as any);
  return new Uint8Array(hash);
}

// Compute CRC-32 for frame integrity
export function crc32(data: Uint8Array): number {
  const table = new Uint32Array(256);
  for (let i = 0; i < 256; i++) {
    let c = i;
    for (let j = 0; j < 8; j++) {
      c = (c & 1) ? (0xEDB88320 ^ (c >>> 1)) : (c >>> 1);
    }
    table[i] = c >>> 0;
  }

  let crc = 0xFFFFFFFF;
  for (let i = 0; i < data.length; i++) {
    crc = table[(crc ^ data[i]) & 0xFF] ^ (crc >>> 8);
  }
  return ((crc ^ 0xFFFFFFFF) >>> 0);
}

// Compute Safety Number for MITM detection
export async function computeSafetyNumber(
  senderPubkey: Uint8Array,
  receiverPubkey: Uint8Array,
  sessionSalt: Uint8Array
): Promise<string> {
  const data = new Uint8Array(96);
  data.set(senderPubkey, 0);
  data.set(receiverPubkey, 32);
  data.set(sessionSalt, 64);

  const hash = await window.crypto.subtle.digest('SHA-256', data as any);
  const hashBytes = new Uint8Array(hash);

  const words = BIP39_WORDS;
  const result: string[] = [];
  for (let i = 0; i < 4; i++) {
    const idx = ((hashBytes[i * 2] << 8) | hashBytes[i * 2 + 1]) % words.length;
    result.push(words[idx]);
  }
  return result.join(' ');
}

// Fountain code seed derivation
export async function deriveFountainSeed(sharedSecret: Uint8Array): Promise<Uint8Array> {
  const baseKey = await window.crypto.subtle.importKey(
    'raw',
    sharedSecret as any,
    'HKDF',
    false,
    ['deriveBits']
  );

  const bits = await window.crypto.subtle.deriveBits(
    {
      name: 'HKDF',
      hash: 'SHA-256',
      salt: new Uint8Array(0),
      info: new TextEncoder().encode('qr-fountain-v1-seed'),
    },
    baseKey,
    256
  );

  return new Uint8Array(bits);
}

// Simple fountain code encoder (XOR-based LT codes)
export class FountainEncoder {
  private sourceBlocks: Uint8Array[];
  private blockSize: number;
  private nextBlockId: number = 0;
  private prng: number = 0;

  constructor(data: Uint8Array, blockSize: number, seed: Uint8Array) {
    this.blockSize = blockSize;
    this.sourceBlocks = [];
    for (let i = 0; i < data.length; i += blockSize) {
      const end = Math.min(i + blockSize, data.length);
      const block = new Uint8Array(blockSize);
      block.set(data.slice(i, end));
      this.sourceBlocks.push(block);
    }
    this.prng = seed.reduce((acc, b) => (((acc * 31 + b) | 0) & 0x7FFFFFFF), 0);
  }

  private nextRandom(max: number): number {
    this.prng = (((this.prng * 1103515245 + 12345) | 0) & 0x7FFFFFFF);
    return this.prng % Math.max(max, 1);
  }

  private degreeDistribution(): number {
    const r = this.nextRandom(100);
    if (r < 10) return 1;
    if (r < 50) return 2;
    if (r < 80) return 3;
    if (r < 95) return 4;
    return Math.min(5 + Math.floor(r / 5), this.sourceBlocks.length);
  }

  nextBlock(): { blockId: number; data: Uint8Array } {
    const degree = Math.min(this.degreeDistribution(), this.sourceBlocks.length);
    const used = new Set<number>();

    for (let i = 0; i < degree; i++) {
      let idx: number;
      do {
        idx = this.nextRandom(this.sourceBlocks.length);
      } while (used.has(idx));
      used.add(idx);
    }

    const indices = Array.from(used);
    const result = new Uint8Array(this.blockSize);
    for (const idx of indices) {
      for (let j = 0; j < this.blockSize; j++) {
        result[j] ^= this.sourceBlocks[idx][j];
      }
    }

    const blockData = new Uint8Array(4 + 4 + indices.length * 4 + this.blockSize);
    const view = new DataView(blockData.buffer, blockData.byteOffset, blockData.byteLength);
    view.setUint32(0, this.nextBlockId, true);
    view.setUint32(4, indices.length, true);
    for (let i = 0; i < indices.length; i++) {
      view.setUint32(8 + i * 4, indices[i], true);
    }
    blockData.set(result, 8 + indices.length * 4);

    const id = this.nextBlockId++;
    return { blockId: id, data: blockData };
  }

  get sourceBlocksCount(): number {
    return this.sourceBlocks.length;
  }

  get sourceSize(): number {
    return this.sourceBlocks.length * this.blockSize;
  }
}

// Simple fountain code decoder
export class FountainDecoder {
  private sourceBlocksCount: number;
  readonly blockSize: number;
  private received: Map<number, { degree: number; indices: number[]; data: Uint8Array }> = new Map();
  private decoded: boolean = false;
  private result: Uint8Array | null = null;
  private totalSourceSize: number;

  constructor(totalSourceSize: number, blockSize: number) {
    this.blockSize = blockSize;
    this.sourceBlocksCount = Math.ceil(totalSourceSize / blockSize);
    this.totalSourceSize = totalSourceSize;
  }

  addBlock(block: { blockId: number; data: Uint8Array }): boolean {
    if (this.decoded) return true;

    const view = new DataView(block.data.buffer, block.data.byteOffset, block.data.byteLength);
    const blockId = view.getUint32(0, true);
    const degree = view.getUint32(4, true);

    const indices: number[] = [];
    for (let i = 0; i < degree; i++) {
      indices.push(view.getUint32(8 + i * 4, true));
    }

    const dataStart = 8 + degree * 4;
    const data = block.data.slice(dataStart, dataStart + this.blockSize);

    this.received.set(blockId, { degree, indices, data });

    const threshold = this.sourceBlocksCount <= 20 ? this.sourceBlocksCount : Math.ceil(this.sourceBlocksCount * 1.05);
    if (this.received.size >= threshold) {
      this.tryDecode();
    }

    return this.decoded;
  }

  private tryDecode(): void {
    const decoded: (Uint8Array | null)[] = new Array(this.sourceBlocksCount).fill(null);

    let progress = true;
    while (progress) {
      progress = false;

      for (const [, block] of this.received) {
        const unknownIndices = block.indices.filter(i => decoded[i] === null);

        if (unknownIndices.length === 1) {
          const idx = unknownIndices[0];
          const result = new Uint8Array(block.data);

          for (const knownIdx of block.indices) {
            if (knownIdx !== idx && decoded[knownIdx] !== null) {
              for (let j = 0; j < this.blockSize; j++) {
                result[j] ^= decoded[knownIdx]![j];
              }
            }
          }

          if (decoded[idx] === null) {
            decoded[idx] = result;
            progress = true;
          }
        }
      }
    }

    if (decoded.every(b => b !== null)) {
      const out = new Uint8Array(this.sourceBlocksCount * this.blockSize);
      for (let i = 0; i < decoded.length; i++) {
        out.set(decoded[i]!, i * this.blockSize);
      }
      this.result = out.slice(0, this.totalSourceSize);
      this.decoded = true;
    }
  }

  getResult(): Uint8Array | null {
    return this.result;
  }

  isComplete(): boolean {
    return this.decoded;
  }

  get blocksReceived(): number {
    return this.received.size;
  }

  get totalSourceBlocks(): number {
    return this.sourceBlocksCount;
  }

  get progress(): number {
    if (this.decoded) return 1;
    return Math.min(this.received.size / (this.sourceBlocksCount * 1.05), 0.99);
  }
}

// Frame format constants
export const FRAME_MAGIC = new Uint8Array([0x51, 0x52]);
export const FRAME_HEADER_SIZE = 22;
export const FRAME_FOOTER_SIZE = 4;
export const MAX_PAYLOAD_SIZE = 1400;

// BIP39 word list
const BIP39_WORDS: string[] = [
  'abandon', 'ability', 'able', 'about', 'above', 'absent', 'absorb', 'abstract',
  'absurd', 'abuse', 'access', 'accident', 'account', 'accuse', 'achieve', 'acid',
  'acoustic', 'acquire', 'across', 'act', 'action', 'actor', 'actress', 'actual',
  'adapt', 'add', 'addict', 'address', 'adjust', 'admit', 'adult', 'advance',
  'advice', 'aerobic', 'affair', 'afford', 'afraid', 'again', 'age', 'agent',
  'agree', 'ahead', 'aim', 'air', 'airport', 'aisle', 'alarm', 'album',
  'alcohol', 'alert', 'alien', 'all', 'alley', 'allow', 'almost', 'alone',
  'alpha', 'already', 'also', 'alter', 'always', 'amateur', 'amazing', 'among',
  'amount', 'amused', 'analyst', 'anchor', 'ancient', 'anger', 'angle', 'angry',
  'animal', 'ankle', 'announce', 'annual', 'another', 'answer', 'antenna', 'antique',
  'anxiety', 'any', 'apart', 'apology', 'appear', 'apple', 'approve', 'april',
  'arch', 'arctic', 'area', 'arena', 'argue', 'arise', 'armor', 'army',
  'around', 'arrange', 'arrest', 'arrive', 'arrow', 'art', 'artefact', 'artist',
  'artwork', 'ask', 'aspect', 'assault', 'asset', 'assist', 'assume', 'asthma',
  'athlete', 'atom', 'attack', 'attend', 'attitude', 'attract', 'auction', 'audit',
  'august', 'aunt', 'author', 'auto', 'autumn', 'average', 'avocado', 'avoid',
  'awake', 'aware', 'away', 'awesome', 'awful', 'awkward', 'axis', 'baby',
  'bachelor', 'bacon', 'badge', 'bag', 'balance', 'balcony', 'ball', 'bamboo',
  'banana', 'banner', 'bar', 'barely', 'bargain', 'barrel', 'base', 'basic',
  'basket', 'battle', 'beach', 'bean', 'beauty', 'because', 'become', 'beef',
  'before', 'begin', 'behave', 'behind', 'believe', 'below', 'belt', 'bench',
  'benefit', 'best', 'betray', 'better', 'between', 'beyond', 'bicycle', 'bid',
  'bike', 'bind', 'biology', 'bird', 'birth', 'bitter', 'black', 'blade',
  'blame', 'blanket', 'blast', 'bleak', 'bless', 'blind', 'blood', 'blossom',
  'blouse', 'blue', 'blur', 'blush', 'board', 'boat', 'body', 'boil',
  'bomb', 'bone', 'bonus', 'book', 'boost', 'border', 'borrow', 'boss',
  'bottom', 'bounce', 'box', 'boy', 'bracket', 'brain', 'brand', 'brass',
  'brave', 'bread', 'breeze', 'brick', 'bridge', 'brief', 'bright', 'brilliant',
  'bring', 'brisk', 'broccoli', 'broken', 'bronze', 'broom', 'brother', 'brown',
  'brush', 'bubble', 'buddy', 'budget', 'buffalo', 'build', 'bulb', 'bulk'
];

// Build frame from components
export function buildFrame(
  chunkIndex: number,
  totalChunks: number,
  fileHashPrefix: Uint8Array,
  keyEpoch: number,
  payload: Uint8Array
): Uint8Array {
  const frame = new Uint8Array(FRAME_HEADER_SIZE + payload.length + FRAME_FOOTER_SIZE);

  frame.set(FRAME_MAGIC, 0);
  frame[2] = 0x01;
  frame[3] = 0x44;

  const view = new DataView(frame.buffer, frame.byteOffset, frame.byteLength);
  view.setUint32(4, chunkIndex, true);
  view.setUint32(8, totalChunks, true);
  frame.set(fileHashPrefix.slice(0, 8), 12);
  view.setUint16(20, keyEpoch, true);
  frame.set(payload, 22);

  const crc = crc32(frame.slice(0, 22 + payload.length));
  view.setUint32(22 + payload.length, crc, true);

  return frame;
}

// Parse frame from bytes
export function parseFrame(data: Uint8Array): {
  chunkIndex: number;
  totalChunks: number;
  fileHashPrefix: Uint8Array;
  keyEpoch: number;
  payload: Uint8Array;
  valid: boolean;
} | null {
  if (data.length < FRAME_HEADER_SIZE + FRAME_FOOTER_SIZE + 1) return null;
  if (data[0] !== 0x51 || data[1] !== 0x52) return null;

  const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
  const chunkIndex = view.getUint32(4, true);
  const totalChunks = view.getUint32(8, true);
  const fileHashPrefix = data.slice(12, 20);
  const keyEpoch = view.getUint16(20, true);
  const payloadEnd = data.length - FRAME_FOOTER_SIZE;
  const payload = data.slice(22, payloadEnd);
  const storedCrc = view.getUint32(payloadEnd, true);

  const computedCrc = crc32(data.slice(0, payloadEnd));
  const valid = storedCrc === computedCrc;

  return { chunkIndex, totalChunks, fileHashPrefix, keyEpoch, payload, valid };
}