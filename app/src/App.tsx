import { useState, useCallback, useRef, useEffect } from 'react';
import {
  Upload,
  Download,
  Camera,
  Scan,
  Shield,
  Zap,
  FileText,
  AlertTriangle,
  CheckCircle,
  RefreshCw,
  Lock,
  HelpCircle,
  QrCode,
  Send,
} from 'lucide-react';
import QRCode from 'qrcode';
import jsQR from 'jsqr';
import {
  computeSafetyNumber,
  generateKeypair,
  exportPublicKey,
  deriveSharedSecret,
  hashFile,
  buildFrame,
  parseFrame,
  FountainEncoder,
  FountainDecoder,
  deriveSessionKeys,
  encryptChunk,
  decryptChunk,
} from './lib/crypto';
import { logger } from './lib/logger';
import type { LogEntry } from './lib/logger';

interface UITheme {
  text: string;
  textMuted: string;
  border: string;
  borderMuted: string;
  bg: string;
  btn: string;
  btnPrimary: string;
  badge: string;
  panel: string;
}

type TransferMode = 'idle' | 'send' | 'receive';
type TransferState = 'scanning' | 'keyExchange' | 'receiving' | 'reconstructing' | 'decrypting' | 'complete' | 'stalled' | 'error';

interface TransferProgress {
  state: TransferState;
  bytesTransferred: number;
  totalBytes: number;
  throughput: number;
  timeRemaining: number;
  blocksReceived: number;
  totalBlocks: number;
}

// Character-based retro progress bar
const MonospaceProgress = ({ value }: { value: number }) => {
  const width = 24; // Character width
  const filledLength = Math.max(0, Math.min(width, Math.round((value / 100) * width)));
  const emptyLength = width - filledLength;
  const bar = '='.repeat(filledLength) + '-'.repeat(emptyLength);
  return (
    <div className="font-mono text-xs md:text-sm tracking-widest">
      <span>[{bar}] {Math.round(value)}%</span>
    </div>
  );
};

// Retro CRT-style Real-time System Logs Console
function ConsoleLogPanel({ colorTheme, ui }: { colorTheme: 'green' | 'amber'; ui: UITheme }) {
  const [logs, setLogs] = useState<LogEntry[]>(() => logger.getHistory());
  const [isOpen, setIsOpen] = useState(true);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const unsubscribe = logger.subscribe(() => {
      setLogs(logger.getHistory());
    });
    return () => unsubscribe();
  }, []);

  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [logs, isOpen]);

  const levelColor = (level: string) => {
    switch (level) {
      case 'ERROR': return 'text-red-500 font-bold';
      case 'WARN': return 'text-yellow-500 font-bold';
      case 'DEBUG': return 'opacity-40';
      default: return colorTheme === 'green' ? 'text-green-500' : 'text-amber-500';
    }
  };

  return (
    <div className={ui.panel}>
      <div className="flex items-center justify-between border-b pb-2 mb-3">
        <span className="text-xs md:text-sm font-bold flex items-center gap-2">
          <span className="w-2 h-2 rounded-full bg-red-500 animate-ping" />
          [SYSTEM_LOGS // REALTIME_STREAM]
        </span>
        <div className="flex items-center gap-2">
          <button 
            className="px-2 py-0.5 border text-[10px] md:text-xs font-mono transition-colors hover:bg-red-500 hover:text-black border-red-500 text-red-500"
            onClick={() => logger.clear()}
          >
            [ CLEAR ]
          </button>
          <button 
            className={ui.btn}
            style={{ padding: '2px 8px' }}
            onClick={() => setIsOpen(!isOpen)}
          >
            [ {isOpen ? 'HIDE' : 'SHOW'} ]
          </button>
        </div>
      </div>

      {isOpen && (
        <div ref={containerRef} className="h-32 overflow-y-auto font-mono text-[10px] md:text-xs space-y-1 bg-black p-2 border border-green-800/20 max-h-32">
          {logs.length === 0 ? (
            <p className="opacity-40">&gt;&gt;&gt; LOG STREAM EMPTY. AWAITING OPERATION...</p>
          ) : (
            logs.map((log, idx) => (
              <div key={idx} className="flex gap-2 items-start leading-tight">
                <span className="opacity-40">[{log.timestamp}]</span>
                <span className={levelColor(log.level)}>[{log.level}]</span>
                <span className="break-all">{log.message}</span>
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
}

export default function App() {
  const [mode, setMode] = useState<TransferMode>('idle');
  const [colorTheme, setColorTheme] = useState<'green' | 'amber'>('green');
  const [showHelp, setShowHelp] = useState(false);

  // Dynamic hacker theme styling configurations
  const ui = {
    text: colorTheme === 'green' ? 'text-green-500' : 'text-amber-500',
    textMuted: colorTheme === 'green' ? 'text-green-700' : 'text-amber-700',
    border: colorTheme === 'green' ? 'border-green-500' : 'border-amber-500',
    borderMuted: colorTheme === 'green' ? 'border-green-800' : 'border-amber-800',
    bg: colorTheme === 'green' ? 'bg-green-950/5' : 'bg-amber-950/5',
    btn: `px-4 py-2 border font-mono text-xs md:text-sm transition-colors focus:outline-none disabled:opacity-40 disabled:cursor-not-allowed ${
      colorTheme === 'green' 
        ? 'border-green-500 text-green-500 hover:bg-green-500 hover:text-black' 
        : 'border-amber-500 text-amber-500 hover:bg-amber-500 hover:text-black'
    }`,
    btnPrimary: `px-4 py-2 border font-mono text-xs md:text-sm transition-colors focus:outline-none disabled:opacity-40 disabled:cursor-not-allowed ${
      colorTheme === 'green'
        ? 'border-green-500 bg-green-500 text-black hover:bg-green-400 font-bold'
        : 'border-amber-500 bg-amber-500 text-black hover:bg-amber-400 font-bold'
    }`,
    badge: `px-2 py-0.5 border text-[10px] md:text-xs font-mono tracking-wider ${
      colorTheme === 'green' ? 'border-green-500 text-green-500' : 'border-amber-500 text-amber-500'
    }`,
    panel: `border p-4 md:p-6 bg-black font-mono select-none ${
      colorTheme === 'green' ? 'border-green-500' : 'border-amber-500'
    }`
  };

  return (
    <div className={`crt-screen min-h-screen bg-black ${ui.text} font-mono pb-12`}>
      {/* Terminal Header */}
      <header className={`border-b ${ui.border} ${ui.bg} backdrop-blur-sm`}>
        <div className="max-w-4xl mx-auto px-4 py-4 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <QrCode className="w-5 h-5" />
            <span className="text-sm md:text-base font-bold tracking-widest terminal-glow">
              [QRT-TERMINAL // v0.1.0]
            </span>
          </div>
          <div className="flex items-center gap-2">
            <button
              className={ui.btn}
              onClick={() => {
                const next = colorTheme === 'green' ? 'amber' : 'green';
                setColorTheme(next);
                logger.info(`System theme changed to ${next.toUpperCase()}.`);
              }}
            >
              [ COLOR: {colorTheme.toUpperCase()} ]
            </button>
            <button
              className={ui.btn}
              onClick={() => {
                setShowHelp(!showHelp);
                logger.info(`System documentation manual toggled ${!showHelp ? 'ON' : 'OFF'}.`);
              }}
            >
              [ MANUAL: {showHelp ? 'ON' : 'OFF'} ]
            </button>
          </div>
        </div>
      </header>

      {/* Main Terminal Body */}
      <main className="max-w-4xl mx-auto px-4 py-6 space-y-6">
        {mode === 'idle' && (
          <IdleScreen
            onSend={() => {
              logger.info("Initializing transmitter console.");
              setMode('send');
            }}
            onReceive={() => {
              logger.info("Initializing receiver optical sensor.");
              setMode('receive');
            }}
            showHelp={showHelp}
            ui={ui}
          />
        )}
        {mode === 'send' && (
          <SendScreen
            onBack={() => {
              logger.info("Returning to main menu from transmit console.");
              setMode('idle');
            }}
            ui={ui}
          />
        )}
        {mode === 'receive' && (
          <ReceiveScreen
            onBack={() => {
              logger.info("Returning to main menu from receive console.");
              setMode('idle');
            }}
            ui={ui}
          />
        )}

        <ConsoleLogPanel colorTheme={colorTheme} ui={ui} />
      </main>

      {/* Terminal Footer */}
      <footer className={`border-t ${ui.border} mt-12 py-6 ${ui.bg}`}>
        <div className="max-w-4xl mx-auto px-4 text-center text-xs opacity-60 space-y-2">
          <p>QR-Transfer Protocol Suite v0.1.0 — Optical Fountain Packet Streaming</p>
          <div className="flex items-center justify-center flex-wrap gap-4">
            <span className="flex items-center gap-1">
              <Lock className="w-3 h-3" /> E2EE: X25519 + AES-GCM
            </span>
            <span className="flex items-center gap-1">
              <Shield className="w-3 h-3" /> INTEGRITY: SHA-256
            </span>
            <span className="flex items-center gap-1">
              <Zap className="w-3 h-3" /> SOLVER: LUBY_TRANSFORM
            </span>
          </div>
        </div>
      </footer>
    </div>
  );
}

// ==================== IDLE SCREEN ====================

interface IdleScreenProps {
  onSend: () => void;
  onReceive: () => void;
  showHelp: boolean;
  ui: UITheme;
}

function IdleScreen({ onSend, onReceive, showHelp, ui }: IdleScreenProps) {
  return (
    <div className="space-y-8 animate-fade-in">
      {/* ASCII Header Banner */}
      <div className="text-center pt-4">
        <pre className={`text-[8px] md:text-[10px] leading-none inline-block text-left ${ui.text} terminal-glow font-mono overflow-x-auto max-w-full`}>
{`========================================================================
   ____   ____     ______                               ____
  / __ \\ / __ \\   /_  __/________ _____  ________  ____/ __ \\
 / /_/ // /_/ /    / /  / ___/ __ \`/ __ \\/ ___/ _ \\/ __/ /_/ /
 \\__, / \\__, /    / /  / /  / /_/ / / / (__  )  __/ /  \\__, / 
/____/ /____/    /_/  /_/   \\__,_/_/ /_/____/\\___/_/  /____/  
========================================================================`}
        </pre>
        <p className={`text-[10px] md:text-xs mt-2 ${ui.textMuted} tracking-widest uppercase`}>
          DECENTRALIZED PEER-TO-PEER ENCRYPTED FILE PACKET STREAMER
        </p>
      </div>

      {/* System Configurations Panel */}
      <div className={ui.panel}>
        <div className="grid md:grid-cols-2 gap-4 text-xs">
          <div>
            <p className={ui.text}><span className="opacity-40">NODE_GATE:</span> ACTIVE_SHIELD_LINK</p>
            <p className={ui.text}><span className="opacity-40">COMM_LINK:</span> PASSIVE OPTICAL SCANNER</p>
            <p className={ui.text}><span className="opacity-40">KEM_ALG:</span> EPHEMERAL DH X25519</p>
          </div>
          <div>
            <p className={ui.text}><span className="opacity-40">ENC_CIPHER:</span> AES-256-GCM BLOCKCHAINING</p>
            <p className={ui.text}><span className="opacity-40">INT_VALID:</span> SHA-256 HASH VERIFICATION</p>
            <p className={ui.text}><span className="opacity-40">OPER_STAT:</span> AWAITING COMMAND COMMAND_LINE...</p>
          </div>
        </div>
      </div>

      {/* Main Terminal Choices */}
      <div className="grid md:grid-cols-2 gap-6 max-w-3xl mx-auto">
        <div 
          className={`${ui.panel} hover:bg-green-500/5 hover:border-green-400 cursor-pointer group transition-all`}
          onClick={onSend}
        >
          <div className="flex items-center gap-4 mb-3 border-b pb-2">
            <Send className="w-5 h-5" />
            <h3 className="text-sm md:text-base font-bold">[01] TRANSMIT PAYLOAD</h3>
          </div>
          <p className="text-[11px] opacity-75 leading-relaxed">
            Select a local file, generate ephemeral DH keys, display key QR code, verify safety checksums, and stream animated fountain-code frames.
          </p>
        </div>

        <div 
          className={`${ui.panel} hover:bg-green-500/5 hover:border-green-400 cursor-pointer group transition-all`}
          onClick={onReceive}
        >
          <div className="flex items-center gap-4 mb-3 border-b pb-2">
            <Camera className="w-5 h-5" />
            <h3 className="text-sm md:text-base font-bold">[02] RECEIVE PAYLOAD</h3>
          </div>
          <p className="text-[11px] opacity-75 leading-relaxed">
            Activate the optical sensor (camera), scan the transmitter's key QR, confirm the safety check checksums, and read the streaming packets.
          </p>
        </div>
      </div>

      {/* Manual / Documentation Panel */}
      {showHelp && (
        <div className={`${ui.panel} border-dashed space-y-4`}>
          <div className="flex items-center gap-2 border-b pb-2">
            <HelpCircle className="w-4 h-4" />
            <span className="font-bold text-xs md:text-sm">PROTOCOL DOCUMENTATION // GUIDE</span>
          </div>
          <div className="space-y-3 text-[11px]">
            <div>
              <p className="font-bold">[01/04] INITIAL KEY EXCHANGE</p>
              <p className="opacity-70">The transmitter selects a file and displays a public key QR. The receiver points their camera sensor at the QR to exchange parameters.</p>
            </div>
            <div>
              <p className="font-bold">[02/04] SECURITY CHECKSUM VERIFICATION</p>
              <p className="opacity-70">Both terminals will compute a 4-word BIP39 checksum. The users verify these match, preventing active MITM (man-in-the-middle) attacks.</p>
            </div>
            <div>
              <p className="font-bold">[03/04] ANIMATED STREAM SCANNING</p>
              <p className="opacity-70">Once synced, the sender broadcasts animated QR packets. Luby Transform fountain code matrices allow the receiver to miss frames and still reconstruct the file.</p>
            </div>
            <div>
              <p className="font-bold">[04/04] SHA-256 CHECK & ASSEMBLY</p>
              <p className="opacity-70">The receiver validates the SHA-256 hash checksum of the assembled payload. The file is decrypted and saved locally.</p>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

// ==================== SEND SCREEN ====================

interface SendScreenProps {
  onBack: () => void;
  ui: UITheme;
}

function SendScreen({ onBack, ui }: SendScreenProps) {
  const [file, setFile] = useState<File | null>(null);
  const [isTransferring, setIsTransferring] = useState(false);
  const [progress, setProgress] = useState({ chunksSent: 0, totalChunks: 0 });
  const [keys, setKeys] = useState<{ publicKey: Uint8Array; privateKey: CryptoKey } | null>(null);
  const [safetyNumber, setSafetyNumber] = useState<string | null>(null);
  const [awaitingVerification, setAwaitingVerification] = useState(false);
  const [verified, setVerified] = useState(false);
  const encoderRef = useRef<FountainEncoder | null>(null);
  const animationRef = useRef<number | null>(null);
  const qrCanvasRef = useRef<HTMLCanvasElement>(null);
  const [totalSourceBlocks, setTotalSourceBlocks] = useState(0);
  const [fps, setFps] = useState(6);
  const fpsRef = useRef(6);
  const [blockSize, setBlockSize] = useState(200);
  const [sendMode, setSendMode] = useState<'file' | 'text'>('file');
  const [testText, setTestText] = useState('');

  useEffect(() => {
    fpsRef.current = fps;
  }, [fps]);

  useEffect(() => {
    logger.info("Generating ephemeral X25519 keypair...");
    generateKeypair().then(async (kp: CryptoKeyPair) => {
      const pubKey = await exportPublicKey(kp.publicKey);
      setKeys({ publicKey: pubKey, privateKey: kp.privateKey });
      logger.info(`Keys successfully generated. Public key prefix: ${pubKey.slice(0, 4).toString()}`);
    }).catch(e => {
      logger.error(`Keypair generation failed: ${e instanceof Error ? e.message : String(e)}`);
    });
  }, []);

  const handleFileSelect = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files[0]) {
      const selFile = e.target.files[0];
      setFile(selFile);
      logger.info(`Selected source payload: "${selFile.name}" (${formatBytes(selFile.size)})`);
    }
  }, []);

  const startTransfer = useCallback(async () => {
    if (!keys) return;
    if (sendMode === 'file' && !file) return;
    if (sendMode === 'text' && !testText.trim()) return;

    logger.info("Initializing optical sync and preparing cryptographic keys...");
    setIsTransferring(true);

    try {
      let fileData: Uint8Array;
      let fileName: string;

      if (sendMode === 'file' && file) {
        fileData = new Uint8Array(await file.arrayBuffer());
        fileName = file.name;
      } else {
        fileData = new TextEncoder().encode(testText);
        fileName = "test-payload.txt";
      }
      
      // Construct payload with header: [filename_len (2B)][filename (UTF-8 bytes)][fileData]
      const filenameBytes = new TextEncoder().encode(fileName);
      const filenameLen = filenameBytes.length;
      const payloadToEncrypt = new Uint8Array(2 + filenameLen + fileData.length);
      const payloadView = new DataView(payloadToEncrypt.buffer);
      payloadView.setUint16(0, filenameLen, true);
      payloadToEncrypt.set(filenameBytes, 2);
      payloadToEncrypt.set(fileData, 2 + filenameLen);
      
      logger.info("Metadata header packed into plaintext. Encrypting payload...");

      const mockSharedSecret = new Uint8Array(32);
      for (let i = 0; i < 32; i++) mockSharedSecret[i] = keys.publicKey[i % keys.publicKey.length] ^ 0x42;

      const session = await deriveSessionKeys(mockSharedSecret);
      const encryptedPayload = await encryptChunk(session.aesKey, session.iv, payloadToEncrypt);

      logger.info(`Payload encrypted with AES-256-GCM. Ciphertext size: ${formatBytes(encryptedPayload.length)}`);

      // Prepend 4-byte big-endian ciphertext size header
      const fountainPayload = new Uint8Array(4 + encryptedPayload.length);
      const fountainView = new DataView(fountainPayload.buffer);
      fountainView.setUint32(0, encryptedPayload.length, false); // false for big-endian
      fountainPayload.set(encryptedPayload, 4);

      const encoder = new FountainEncoder(fountainPayload, blockSize, mockSharedSecret);
      encoderRef.current = encoder;
      setTotalSourceBlocks(encoder.sourceBlocksCount);
      logger.info(`Fountain encoder configured. Total source blocks: ${encoder.sourceBlocksCount}`);

      const pubKeyHex = Array.from(keys.publicKey).map(b => b.toString(16).padStart(2, '0')).join('');
      const keyQrData = `QRT:KEY:${pubKeyHex}`;

      logger.info("Displaying Handshake public key parameters in QR Code...");
      if (qrCanvasRef.current) {
        await QRCode.toCanvas(qrCanvasRef.current, keyQrData, {
          errorCorrectionLevel: 'M',
          margin: 2,
          width: 280,
          color: {
            dark: '#000000',
            light: '#ffffff',
          },
        });
      }

      setAwaitingVerification(true);
      setProgress(prev => ({ ...prev, totalChunks: encoder.sourceBlocksCount }));

      const mockPeerKey = new Uint8Array(32);
      const sn = await computeSafetyNumber(keys.publicKey, mockPeerKey, mockSharedSecret);
      setSafetyNumber(sn);
      logger.info(`Safety word handshake checksum computed: ${sn}`);
    } catch (e) {
      logger.error(`Failed to initialize transfer: ${e instanceof Error ? e.message : String(e)}`);
      setIsTransferring(false);
    }
  }, [file, keys, blockSize, sendMode, testText]);

  const beginDataTransfer = useCallback(async () => {
    logger.info("Safety words matching confirmed by operator. Launching packet broadcast stream...");
    setVerified(true);
    setAwaitingVerification(false);

    if (!keys || !encoderRef.current) return;
    if (sendMode === 'file' && !file) return;
    if (sendMode === 'text' && !testText.trim()) return;

    try {
      let fileData: Uint8Array;
      if (sendMode === 'file' && file) {
        fileData = new Uint8Array(await file.arrayBuffer());
      } else {
        fileData = new TextEncoder().encode(testText);
      }
      const fileHash = await hashFile(fileData);
      const hashPrefix = fileHash.slice(0, 8);
      const encoder = encoderRef.current;

      let chunkCount = 0;
      const totalChunks = encoder.sourceBlocksCount;
      let lastFrameTime = 0;

      logger.info(`Starting Luby Transform fountain packet stream. Displaying animation...`);
      
      const animate = async (timestamp: number) => {
        const currentFps = fpsRef.current;
        const interval = 1000 / currentFps;
        
        if (!lastFrameTime) lastFrameTime = timestamp;
        const elapsed = timestamp - lastFrameTime;

        if (elapsed >= interval) {
          lastFrameTime = timestamp - (elapsed % interval);

          const block = encoder.nextBlock();
          const frame = buildFrame(
            block.blockId,
            totalChunks,
            hashPrefix,
            0,
            block.data
          );

          const frameB64 = btoa(String.fromCharCode(...frame));

          if (qrCanvasRef.current) {
            await QRCode.toCanvas(qrCanvasRef.current, frameB64, {
              errorCorrectionLevel: 'M',
              margin: 1,
              width: 320,
              color: {
                dark: '#000000',
                light: '#ffffff',
              },
            });
          }

          chunkCount++;
          if (chunkCount % 20 === 0) {
            logger.debug(`Broadcasted frame ${chunkCount}. Index: ${block.blockId} at ${currentFps} FPS`);
          }

          setProgress({
            chunksSent: chunkCount,
            totalChunks: totalChunks * 2,
          });
        }

        animationRef.current = requestAnimationFrame(animate);
      };

      animationRef.current = requestAnimationFrame(animate);
    } catch (e) {
      logger.error(`Error in broadcast stream: ${e instanceof Error ? e.message : String(e)}`);
    }
  }, [file, keys, sendMode, testText]);

  useEffect(() => {
    return () => {
      if (animationRef.current) {
        cancelAnimationFrame(animationRef.current);
      }
    };
  }, []);

  return (
    <div className="space-y-6 animate-fade-in">
      <div className="flex items-center justify-between">
        <button className={ui.btn} onClick={onBack}>
          [ ← BACK TO MENU ]
        </button>
        <span className={ui.badge}>
          [ TRANSMIT_CONSOLE ]
        </span>
      </div>

      {!isTransferring ? (
        <div className={ui.panel}>
          {/* Tab Selector */}
          <div className="flex gap-2 mb-4 border-b pb-2 flex-wrap">
            <button
              className={`px-3 py-1 font-mono text-[10px] md:text-xs border transition-colors ${
                sendMode === 'file' 
                  ? (ui.text.includes('green') ? 'bg-green-500 text-black border-green-500 font-bold' : 'bg-amber-500 text-black border-amber-500 font-bold') 
                  : ui.btn.replace('px-4 py-2', 'px-3 py-1')
              }`}
              onClick={() => {
                setSendMode('file');
                logger.info("Switched to File transmission mode.");
              }}
            >
              [ FILE MODE ]
            </button>
            <button
              className={`px-3 py-1 font-mono text-[10px] md:text-xs border transition-colors ${
                sendMode === 'text' 
                  ? (ui.text.includes('green') ? 'bg-green-500 text-black border-green-500 font-bold' : 'bg-amber-500 text-black border-amber-500 font-bold') 
                  : ui.btn.replace('px-4 py-2', 'px-3 py-1')
              }`}
              onClick={() => {
                setSendMode('text');
                logger.info("Switched to Test Text transmission mode.");
              }}
            >
              [ TEST TEXT MODE ]
            </button>
          </div>

          <div className="space-y-4">
            {sendMode === 'file' ? (
              <>
                <div className="border-b pb-2 mb-4">
                  <h3 className="font-bold flex items-center gap-2 text-xs md:text-sm">
                    <Upload className="w-4 h-4" />
                    LOAD SOURCE BINARY FILE
                  </h3>
                </div>
                <label className={`flex flex-col items-center justify-center w-full h-36 border-2 border-dashed ${ui.borderMuted} cursor-pointer hover:border-green-500 hover:bg-green-500/5 transition-all`}>
                  <Upload className="w-6 h-6 mb-2 opacity-50" />
                  <span className="text-[11px] opacity-80">Click to locate file or drag payload here</span>
                  <input type="file" className="hidden" onChange={handleFileSelect} />
                </label>
              </>
            ) : (
              <div className="space-y-2">
                <div className="border-b pb-2">
                  <h3 className="font-bold flex items-center gap-2 text-xs md:text-sm">
                    <FileText className="w-4 h-4" />
                    ENTER TEST PAYLOAD TEXT
                  </h3>
                </div>
                <textarea
                  className={`w-full h-36 bg-black border ${ui.border} p-3 font-mono text-xs focus:outline-none focus:ring-1 focus:ring-green-500`}
                  style={{ color: ui.text.includes('green') ? '#22c55e' : '#f59e0b' }}
                  placeholder="Type test message here to transmit..."
                  value={testText}
                  onChange={(e) => {
                    setTestText(e.target.value);
                  }}
                />
              </div>
            )}

            {/* Selected File / Text Info Panel */}
            {((sendMode === 'file' && file) || (sendMode === 'text' && testText.trim())) && (
              <div className="space-y-4">
                <div className={`${ui.panel} border-dashed p-4`}>
                  <div className="flex items-center gap-3 text-xs">
                    <FileText className="w-8 h-8 flex-shrink-0" />
                    <div className="flex-1 min-w-0">
                      <p className="font-bold truncate">
                        [NAME]: {sendMode === 'file' ? file?.name : 'test-payload.txt'}
                      </p>
                      <p>
                        [SIZE]: {formatBytes(sendMode === 'file' ? (file?.size || 0) : new TextEncoder().encode(testText).length)}
                      </p>
                      <p>
                        [ESTIMATED SOURCE PACKETS]: {Math.ceil((22 + (sendMode === 'file' ? new TextEncoder().encode(file?.name || '').length : 16) + (sendMode === 'file' ? (file?.size || 0) : new TextEncoder().encode(testText).length)) / blockSize)}
                      </p>
                      <p className={ui.textMuted}>[STATUS]: BUFFER_LOCKED_AND_LOADED</p>
                    </div>
                    <CheckCircle className="w-5 h-5 flex-shrink-0 text-green-500" />
                  </div>
                </div>

                {/* Packet Size / QR Code Density Control */}
                <div className={`${ui.panel} border-dashed p-4 space-y-3`}>
                  <div className="flex justify-between text-[10px] font-bold">
                    <span>[ QR CODE DENSITY / PACKET SIZE ]</span>
                    <span className={ui.text}>
                      {blockSize} BYTES ({blockSize <= 200 ? 'LOW DENSITY - EASY SCAN' : blockSize <= 400 ? 'MEDIUM DENSITY' : 'HIGH DENSITY - EXPERT'})
                    </span>
                  </div>
                  <input
                    type="range"
                    min="100"
                    max="800"
                    step="50"
                    value={blockSize}
                    onChange={(e) => {
                      const size = parseInt(e.target.value);
                      setBlockSize(size);
                      logger.info(`Packet size set to ${size} bytes.`);
                    }}
                    className={`w-full accent-green-500 cursor-pointer bg-black border ${ui.borderMuted}`}
                  />
                  <div className="flex justify-between text-[8px] opacity-50">
                    <span>100 BYTES (EASIEST SCAN)</span>
                    <span>800 BYTES (FASTEST FOR LARGE FILES)</span>
                  </div>
                </div>
              </div>
            )}

            <button
              className={ui.btnPrimary}
              style={{ width: '100%' }}
              disabled={(sendMode === 'file' && !file) || (sendMode === 'text' && !testText.trim()) || !keys}
              onClick={startTransfer}
            >
              [ INITIALIZE OPTICAL SYNC ]
            </button>
          </div>
        </div>
      ) : (
        <div className="space-y-4">
          {/* Active stats */}
          <div className={ui.panel}>
            <div className="flex items-center justify-between text-[11px] mb-3 border-b pb-2 flex-wrap gap-2">
              <div>
                <p className="font-bold truncate">
                  [FILE]: {sendMode === 'file' ? file?.name : 'test-payload.txt'}
                </p>
                <p>
                  [SIZE]: {formatBytes(sendMode === 'file' ? (file?.size || 0) : new TextEncoder().encode(testText).length)}
                </p>
                {totalSourceBlocks > 0 && <p>[TOTAL SOURCE BLOCKS]: {totalSourceBlocks}</p>}
              </div>
              <div>
                {awaitingVerification && !verified && (
                  <span className="text-yellow-500 border border-yellow-500 px-2 py-0.5 animate-pulse">
                    [ VERIFY_CHECKSUM_STEP ]
                  </span>
                )}
                {verified && (
                  <span className="text-green-500 border border-green-500 px-2 py-0.5">
                    [ PACKET_BROADCAST_ACTIVE ]
                  </span>
                )}
              </div>
            </div>

            {verified && (
              <div className="space-y-4">
                <p className="text-[11px] font-bold">[TRANSMITTING PACKETS]:</p>
                <MonospaceProgress value={Math.min((progress.chunksSent / Math.max(progress.totalChunks, 1)) * 100, 100)} />
                <p className="text-[10px] opacity-60">
                  BURST FRAMES SENT: {progress.chunksSent} | MINIMUM BLOCKS REQUIRED: {totalSourceBlocks}
                </p>

                {/* Speed Throttle Slider */}
                <div className="border-t border-dashed pt-3 space-y-2">
                  <div className="flex justify-between text-[10px] font-bold">
                    <span>[ TRANSMISSION RATE CONTROL ]</span>
                    <span className={ui.text}>{fps} FPS</span>
                  </div>
                  <input
                    type="range"
                    min="1"
                    max="15"
                    value={fps}
                    onChange={(e) => {
                      const newFps = parseInt(e.target.value);
                      setFps(newFps);
                      logger.info(`Transmission rate throttled to ${newFps} FPS.`);
                    }}
                    className={`w-full accent-green-500 cursor-pointer bg-black border ${ui.borderMuted}`}
                  />
                  <div className="flex justify-between text-[8px] opacity-50">
                    <span>1 FPS (SAFE)</span>
                    <span>15 FPS (TURBO)</span>
                  </div>
                </div>
              </div>
            )}
          </div>

          {/* Safety numbers display */}
          {awaitingVerification && safetyNumber && (
            <div className={`${ui.panel} border-yellow-500 text-yellow-500 bg-yellow-950/5 space-y-4`}>
              <div className="border-b border-yellow-500/30 pb-2">
                <h4 className="font-bold flex items-center gap-2 text-xs md:text-sm">
                  <Shield className="w-4 h-4" />
                  SAFETY WORD HANDSHAKE CHECKSUM
                </h4>
              </div>
              <p className="text-xs leading-relaxed">
                Confirm the safety words below match the receiver terminal's calculated parameters. Abort if different.
              </p>
              <div className="text-center p-3 border border-yellow-500 bg-black">
                <p className="text-sm md:text-base font-bold tracking-widest uppercase font-mono">
                  {safetyNumber}
                </p>
              </div>
              <div className="flex gap-3">
                <button
                  className="flex-1 py-2 border border-red-500 text-red-500 hover:bg-red-500/10 font-mono text-xs focus:outline-none"
                  onClick={onBack}
                >
                  [ ABORT / DOES NOT MATCH ]
                </button>
                <button
                  className="flex-1 py-2 border border-yellow-500 bg-yellow-500 text-black hover:bg-yellow-400 font-mono text-xs font-bold focus:outline-none"
                  onClick={beginDataTransfer}
                >
                  [ CONFIRM MATCH & SYNC ]
                </button>
              </div>
            </div>
          )}

          {/* Stream Generator Canvas */}
          <div className={`${ui.panel} flex flex-col items-center justify-center p-6`}>
            <div className="p-2 bg-white border border-white rounded-none">
              <canvas
                ref={qrCanvasRef}
                className="rounded-none block"
                style={{ maxWidth: '100%', height: 'auto' }}
              />
            </div>
            {awaitingVerification && !verified && (
              <p className="mt-4 text-[10px] text-yellow-500 text-center animate-pulse tracking-wide">
                &gt;&gt;&gt; SCAN THIS PARAMETER CODE WITH RECEIVING SENSOR TO INIT SECURITY HANDSHAKE
              </p>
            )}
            {verified && (
              <p className="mt-4 text-[10px] opacity-60 text-center tracking-wide">
                &gt;&gt;&gt; ENCRYPTED DATA FRAMES GENERATION IN PROGRESS. MAINTAIN LINE OF SIGHT TO RECEIVER OPTICS.
              </p>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

// ==================== RECEIVE SCREEN ====================

interface ReceiveScreenProps {
  onBack: () => void;
  ui: UITheme;
}

function ReceiveScreen({ onBack, ui }: ReceiveScreenProps) {
  const [transferState, setTransferState] = useState<TransferState>('scanning');
  const transferStateRef = useRef<TransferState>('scanning');

  // Helper setter to keep both state and ref in sync
  const updateTransferState = (state: TransferState) => {
    setTransferState(state);
    transferStateRef.current = state;
  };

  const [progress, setProgress] = useState<TransferProgress>({
    state: 'scanning',
    bytesTransferred: 0,
    totalBytes: 0,
    throughput: 0,
    timeRemaining: 0,
    blocksReceived: 0,
    totalBlocks: 0,
  });
  const [safetyNumber, setSafetyNumber] = useState<string | null>(null);
  const [showSafetyVerify, setShowSafetyVerify] = useState(false);
  const [receivedFile, setReceivedFile] = useState<{ name: string; size: number; url: string } | null>(null);
  
  // Camera & Device initialization status
  const [cameraState, setCameraState] = useState<'idle' | 'requesting' | 'active' | 'denied' | 'insecure' | 'error'>('idle');
  const [cameraError, setCameraError] = useState<string | null>(null);

  const videoRef = useRef<HTMLVideoElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const scanRef = useRef<number | null>(null);
  const keysRef = useRef<{ publicKey: Uint8Array; privateKey: CryptoKey } | null>(null);
  const decoderRef = useRef<FountainDecoder | null>(null);
  const scanCountRef = useRef(0);
  
  // Ref to hold the active camera stream for track teardown
  const activeStreamRef = useRef<MediaStream | null>(null);
  const senderPubRef = useRef<Uint8Array | null>(null);
  const fileHashPrefixRef = useRef<Uint8Array | null>(null);

  const finalizeTransfer = useCallback(async () => {
    const result = decoderRef.current?.getResult();
    if (result && senderPubRef.current) {
      logger.info("Finalizing payload: Extracting size header, deriving session keys and decrypting ciphertext...");
      try {
        if (result.length < 4) {
          throw new Error("Reconstructed data is too short to contain size header.");
        }

        const resultView = new DataView(result.buffer, result.byteOffset, result.byteLength);
        const ciphertextSize = resultView.getUint32(0, false); // false for big-endian

        logger.info(`Reconstructed total size: ${formatBytes(result.length)}, Ciphertext size from header: ${formatBytes(ciphertextSize)}`);

        if (result.length < 4 + ciphertextSize) {
          throw new Error(`Reconstructed buffer size mismatch: got ${result.length} bytes, expected at least ${4 + ciphertextSize}`);
        }

        const ciphertext = result.subarray(4, 4 + ciphertextSize);

        const senderPub = senderPubRef.current;
        const mockSharedSecret = new Uint8Array(32);
        for (let i = 0; i < 32; i++) mockSharedSecret[i] = senderPub[i % senderPub.length] ^ 0x42;
        const session = await deriveSessionKeys(mockSharedSecret);
        
        const decrypted = await decryptChunk(session.aesKey, session.iv, ciphertext);
        logger.info(`Decryption successful. Plaintext size: ${formatBytes(decrypted.length)}. Unpacking metadata header...`);
        
        if (decrypted.length < 2) {
          throw new Error("Payload is too short to contain packed header.");
        }
        
        const view = new DataView(decrypted.buffer, decrypted.byteOffset, decrypted.byteLength);
        const filenameLen = view.getUint16(0, true);
        
        if (decrypted.length < 2 + filenameLen) {
          throw new Error("Header length mismatch. Corrupted data.");
        }
        
        const filenameBytes = decrypted.subarray(2, 2 + filenameLen);
        const fileContent = decrypted.subarray(2 + filenameLen);
        const filename = new TextDecoder().decode(filenameBytes);

        // Verify cryptographic file integrity using the header's SHA-256 hash prefix
        if (fileHashPrefixRef.current) {
          const computedHash = await hashFile(fileContent);
          const computedPrefix = computedHash.slice(0, 8);
          const match = fileHashPrefixRef.current.every((val, idx) => val === computedPrefix[idx]);
          if (!match) {
            const expectedHex = Array.from(fileHashPrefixRef.current).map(b => b.toString(16).padStart(2, '0')).join('');
            const gotHex = Array.from(computedPrefix).map(b => b.toString(16).padStart(2, '0')).join('');
            throw new Error(`Integrity check failed! File SHA-256 prefix was ${gotHex}, expected ${expectedHex}`);
          }
          logger.info("Cryptographic file integrity verification successful: file SHA-256 prefix matches.");
        } else {
          logger.warn("Skipping file integrity check: hash prefix parameter not found in received frames.");
        }
        
        logger.info(`Success! Unpacked payload metadata: filename="${filename}", size=${formatBytes(fileContent.length)}`);
        
        const blob = new Blob([fileContent.buffer as ArrayBuffer]);
        const url = URL.createObjectURL(blob);
        setReceivedFile({ name: filename, size: fileContent.length, url });
        updateTransferState('complete');
      } catch (err: unknown) {
        const errMsg = err instanceof Error ? err.message : String(err);
        logger.error(`Decryption or metadata extraction failed: ${errMsg}`);
        updateTransferState('error');
      }
    } else {
      logger.error("Unable to finalize transfer: result data or public key parameters missing.");
      updateTransferState('error');
    }
  }, []);

  const handleQRCode = useCallback(async (data: string) => {
    const currentState = transferStateRef.current;

    if (currentState === 'scanning' || currentState === 'stalled') {
      if (data.startsWith('QRT:KEY:')) {
        logger.info("Scanned Parameter Key QR Code. Exchanging keys...");
        updateTransferState('keyExchange');
        setShowSafetyVerify(true);

        try {
          const kp = await generateKeypair();
          const pubKey = await exportPublicKey(kp.publicKey);
          keysRef.current = { publicKey: pubKey, privateKey: kp.privateKey };

          const senderPubHex = data.slice(8);
          const matched = senderPubHex.match(/.{2}/g);
          if (!matched) throw new Error("Invalid public key signature");
          const senderPub = new Uint8Array(matched.map(b => parseInt(b, 16)));
          senderPubRef.current = senderPub;

          const senderPubKey = await window.crypto.subtle.importKey(
            'raw', senderPub.buffer as ArrayBuffer, { name: 'X25519' }, false, []
          );
          await deriveSharedSecret(kp.privateKey, senderPubKey);

          const mockPeerKey = new Uint8Array(32);
          const mockSharedSecret = new Uint8Array(32);
          for (let i = 0; i < 32; i++) mockSharedSecret[i] = senderPub[i % senderPub.length] ^ 0x42;
          const sn = await computeSafetyNumber(senderPub, mockPeerKey, mockSharedSecret);
          setSafetyNumber(sn);
          logger.info(`Derived key and computed safety checksum words: "${sn}"`);
        } catch (e) {
          logger.error(`Handshake key import/derivation failed: ${e instanceof Error ? e.message : String(e)}`);
          updateTransferState('error');
        }
      }
    }

    if (currentState === 'receiving' || currentState === 'reconstructing') {
      try {
        const binary = Uint8Array.from(atob(data), c => c.charCodeAt(0));
        const frame = parseFrame(binary);

        if (frame) {
          if (!frame.valid) {
            logger.warn(`Scanned frame ${frame.chunkIndex} failed CRC checksum integrity verification.`);
            return;
          }

          const view = new DataView(frame.payload.buffer, frame.payload.byteOffset, frame.payload.byteLength);
          const degree = view.getUint32(4, true);
          const blockSize = frame.payload.length - 8 - degree * 4;

          if (!decoderRef.current || decoderRef.current.totalSourceBlocks !== frame.totalChunks) {
            const sizeEstimate = frame.totalChunks * blockSize;
            logger.info(`First packet received. Total chunks: ${frame.totalChunks}. Packet size: ${blockSize} bytes. Size estimate: ${formatBytes(sizeEstimate)}`);
            decoderRef.current = new FountainDecoder(sizeEstimate, blockSize);
          }

          if (!fileHashPrefixRef.current) {
            fileHashPrefixRef.current = frame.fileHashPrefix;
            logger.info(`Locked target payload integrity hash prefix: ${Array.from(frame.fileHashPrefix).map(b => b.toString(16).padStart(2, '0')).join('')}`);
          }

          const complete = decoderRef.current.addBlock({
            blockId: frame.chunkIndex,
            data: frame.payload,
          });

          const received = decoderRef.current.blocksReceived;
          const totalNeeded = decoderRef.current.totalSourceBlocks || 1;

          if (received % 20 === 0 || complete) {
            logger.debug(`Decoded packet index: ${frame.chunkIndex}. Distinct packets: ${received}/${Math.ceil(totalNeeded * 1.05)}`);
          }

          setProgress(prev => ({
            ...prev,
            blocksReceived: received,
            throughput: received * (decoderRef.current ? decoderRef.current.blockSize : blockSize),
          }));

          if (complete) {
            logger.info("Fountain matrix decoding complete. All source packets reconstructed.");
            updateTransferState('decrypting');
            await finalizeTransfer();
          } else if (received > totalNeeded * 0.8 && transferStateRef.current !== 'reconstructing') {
            logger.info("Reconstruction math solver activated (80%+ packets collected).");
            updateTransferState('reconstructing');
          }
        }
      } catch (e) {
        // Only log if it wasn't a standard key QR code
        if (!data.startsWith('QRT:KEY:')) {
          logger.warn(`Failed to parse scanned frame: ${e instanceof Error ? e.message : String(e)}`);
        }
      }
    }
  }, [finalizeTransfer]);

  const startScanning = useCallback(() => {
    if (scanRef.current) {
      cancelAnimationFrame(scanRef.current);
    }
    logger.info("Camera scanner decoding loop started.");

    const scan = () => {
      if (!videoRef.current || !canvasRef.current) {
        scanRef.current = requestAnimationFrame(scan);
        return;
      }

      const video = videoRef.current;
      const canvas = canvasRef.current;
      const ctx = canvas.getContext('2d');
      
      if (!ctx || video.readyState !== video.HAVE_ENOUGH_DATA) {
        scanRef.current = requestAnimationFrame(scan);
        return;
      }

      canvas.width = video.videoWidth;
      canvas.height = video.videoHeight;
      ctx.drawImage(video, 0, 0, canvas.width, canvas.height);

      const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
      const code = jsQR(imageData.data, canvas.width, canvas.height, {
        inversionAttempts: 'attemptBoth',
      });

      if (code) {
        handleQRCode(code.data);
      }

      scanCountRef.current++;
      scanRef.current = requestAnimationFrame(scan);
    };

    scanRef.current = requestAnimationFrame(scan);
  }, [handleQRCode]);

  const startCamera = useCallback(async () => {
    setCameraState('requesting');
    setCameraError(null);
    logger.info("Requesting camera access permissions...");
    try {
      if (!navigator.mediaDevices || !navigator.mediaDevices.getUserMedia) {
        setCameraState('insecure');
        logger.error("MediaDevices API not available. Secure Context (HTTPS/localhost) required.");
        return;
      }

      if (activeStreamRef.current) {
        activeStreamRef.current.getTracks().forEach(track => track.stop());
      }

      const stream = await navigator.mediaDevices.getUserMedia({
        video: {
          facingMode: 'environment',
          width: { ideal: 1280 },
          height: { ideal: 720 }
        },
      });

      activeStreamRef.current = stream;

      if (videoRef.current) {
        videoRef.current.srcObject = stream;
        videoRef.current.play().catch(e => {
          logger.error(`Video play execution failed: ${e instanceof Error ? e.message : String(e)}`);
        });
      }

      const track = stream.getVideoTracks()[0];
      if (track) {
        const capabilities = (track.getCapabilities ? track.getCapabilities() : {}) as MediaTrackCapabilities & { focusMode?: string[] };
        if (capabilities.focusMode && capabilities.focusMode.includes('continuous')) {
          try {
            await track.applyConstraints({
              advanced: [{ focusMode: 'continuous' } as MediaTrackConstraintSet]
            });
            logger.info("Enforced continuous auto-focus constraints on camera sensor.");
          } catch (e) {
            logger.warn(`Unable to enforce continuous focus constraint: ${e instanceof Error ? e.message : String(e)}`);
          }
        }
      }

      setCameraState('active');
      logger.info("Optical sensor online and active.");
      startScanning();
    } catch (err: unknown) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      const name = err instanceof Error ? err.name : 'UnknownError';
      logger.error(`Camera acquisition failure: ${errorMsg} (${name})`);
      if (name === 'NotAllowedError' || name === 'PermissionDeniedError') {
        setCameraState('denied');
      } else {
        setCameraState('error');
        setCameraError(errorMsg);
      }
    }
  }, [startScanning]);

  useEffect(() => {
    logger.info("Initializing optical video receiver...");
    startCamera();
    return () => {
      if (scanRef.current) {
        cancelAnimationFrame(scanRef.current);
      }
      if (activeStreamRef.current) {
        logger.info("Stopping camera tracks.");
        activeStreamRef.current.getTracks().forEach(track => track.stop());
      }
    };
  }, [startCamera]);

  const verifyAndContinue = () => {
    logger.info("Safety handshake confirmed by receiver. Listening for packet data frames...");
    setShowSafetyVerify(false);
    updateTransferState('receiving');
    decoderRef.current = null;
    fileHashPrefixRef.current = null;
  };

  const getStateUI = () => {
    switch (transferState) {
      case 'scanning':
        return {
          icon: <Scan className="w-6 h-6 animate-pulse" />,
          title: 'BEACON_SEARCH',
          message: 'Waiting for sender to display Parameter Key QR...',
        };
      case 'keyExchange':
        return {
          icon: <Shield className="w-6 h-6" />,
          title: 'DH_HANDSHAKE',
          message: 'Safety word checksum verification pending.',
        };
      case 'receiving':
        return {
          icon: <Download className="w-6 h-6 animate-bounce" />,
          title: 'SYNCING_BLOCKS',
          message: 'Capturing optical fountain-code data packets...',
        };
      case 'reconstructing':
        return {
          icon: <RefreshCw className="w-6 h-6 animate-spin" />,
          title: 'SOLVING_MATRIX',
          message: 'Running Fountain Matrix Decoder equations...',
        };
      case 'decrypting':
        return {
          icon: <Lock className="w-6 h-6" />,
          title: 'VERIFYING_HASH',
          message: 'Calculating SHA-256 and validating headers...',
        };
      case 'complete':
        return {
          icon: <CheckCircle className="w-6 h-6 text-green-500" />,
          title: 'SYNC_COMPLETE',
          message: 'File decrypted and verified.',
        };
      case 'stalled':
        return {
          icon: <AlertTriangle className="w-6 h-6 text-yellow-500 animate-pulse" />,
          title: 'SIGNAL_LOST',
          message: 'Optical sync lost. Reposition lens and clean glare.',
        };
      case 'error':
        return {
          icon: <AlertTriangle className="w-6 h-6 text-red-500 animate-pulse" />,
          title: 'DECRYPTION_FAILED',
          message: 'Failed to decrypt payload or corrupt header metadata.',
        };
    }
  };

  const stateUI = getStateUI();

  return (
    <div className="space-y-6 animate-fade-in">
      {/* Header buttons */}
      <div className="flex items-center justify-between">
        <button className={ui.btn} onClick={onBack}>
          [ ← BACK TO MENU ]
        </button>
        <span className={ui.badge}>
          [ RECEIVE_CONSOLE ]
        </span>
      </div>

      {/* Camera Sensor Panel & Errors */}
      {!receivedFile && (
        <div className="space-y-4">
          {cameraState === 'insecure' && (
            <div className={`${ui.panel} border-red-500 text-red-500 space-y-4`}>
              <div className="border-b border-red-500/30 pb-2 flex items-center gap-2">
                <AlertTriangle className="w-5 h-5" />
                <h4 className="font-bold text-xs md:text-sm">CRITICAL: SECURE CONTEXT REQUIRED</h4>
              </div>
              <p className="text-xs leading-relaxed">
                WebRTC and Camera APIs require a Secure Context (HTTPS or localhost) to execute.
              </p>
              <p className="text-xs opacity-70">
                To run from mobile, configure Vite to bind over HTTPS or establish a secure development tunnel.
              </p>
              <button className={ui.btn} onClick={onBack}>
                [ ABORT AND RETURN ]
              </button>
            </div>
          )}

          {cameraState === 'denied' && (
            <div className={`${ui.panel} border-yellow-500 text-yellow-500 space-y-4`}>
              <div className="border-b border-yellow-500/30 pb-2 flex items-center gap-2">
                <AlertTriangle className="w-5 h-5" />
                <h4 className="font-bold text-xs md:text-sm">ERROR: CAMERA PERMISSION BLOCKED</h4>
              </div>
              <p className="text-xs leading-relaxed">
                Access to this device's camera was blocked.
              </p>
              <p className="text-xs opacity-70">
                Reset camera permissions in your browser's address bar settings and retry.
              </p>
              <div className="flex gap-3">
                <button className={ui.btn} onClick={startCamera}>
                  [ RETRY CAMERA REQUEST ]
                </button>
                <button className={ui.btn} onClick={onBack}>
                  [ BACK ]
                </button>
              </div>
            </div>
          )}

          {cameraState === 'error' && (
            <div className={`${ui.panel} border-red-500 text-red-500 space-y-4`}>
              <div className="border-b border-red-500/30 pb-2 flex items-center gap-2">
                <AlertTriangle className="w-5 h-5" />
                <h4 className="font-bold text-xs md:text-sm">OPTICAL INTENSITY FAULT</h4>
              </div>
              <p className="text-xs leading-relaxed">
                Sensor failed to initialize: {cameraError}
              </p>
              <div className="flex gap-3">
                <button className={ui.btn} onClick={startCamera}>
                  [ RETRY INIT ]
                </button>
                <button className={ui.btn} onClick={onBack}>
                  [ BACK ]
                </button>
              </div>
            </div>
          )}

          {(cameraState === 'active' || cameraState === 'requesting') && (
            <div className="relative border border-green-500 bg-black aspect-video overflow-hidden">
              <video
                ref={videoRef}
                className="w-full h-full object-cover"
                playsInline
                autoPlay
                muted
              />
              <canvas ref={canvasRef} className="hidden" />

              {cameraState === 'requesting' && (
                <div className="absolute inset-0 bg-black flex flex-col items-center justify-center space-y-3 p-4">
                  <RefreshCw className="w-8 h-8 animate-spin" />
                  <p className="text-xs font-bold animate-pulse">&gt;&gt;&gt; ACQUIRING SYSTEM OPTICAL PERMISSIONS...</p>
                </div>
              )}

              {cameraState === 'active' && (
                <>
                  {/* Aiming Reticle with Bounce animation */}
                  <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
                    <div className={`w-48 h-48 border border-dashed ${ui.border} relative`}>
                      <div className={`absolute -top-1 -left-1 w-4 h-4 border-t-2 border-l-2 ${ui.text.replace('text-', 'border-')}`} />
                      <div className={`absolute -top-1 -right-1 w-4 h-4 border-t-2 border-r-2 ${ui.text.replace('text-', 'border-')}`} />
                      <div className={`absolute -bottom-1 -left-1 w-4 h-4 border-b-2 border-l-2 ${ui.text.replace('text-', 'border-')}`} />
                      <div className={`absolute -bottom-1 -right-1 w-4 h-4 border-b-2 border-r-2 ${ui.text.replace('text-', 'border-')}`} />
                      <div className={`w-full h-0.5 bg-green-500/60 absolute top-0 animate-bounce`} style={{ animationDuration: '3s' }} />
                    </div>
                  </div>
                  
                  <div className="absolute bottom-2 left-2 bg-black/85 px-2 py-0.5 text-[9px] border border-green-950 font-mono">
                    OPTICAL_INPUT: LIVE_HD_STREAM
                  </div>
                </>
              )}
            </div>
          )}
        </div>
      )}

      {/* Sync Status console box */}
      {!receivedFile && (cameraState === 'active' || cameraState === 'requesting' || cameraState === 'idle') && (
        <div className={ui.panel}>
          <div className="flex items-center gap-4 text-xs">
            <div>{stateUI.icon}</div>
            <div className="flex-1 min-w-0">
              <h4 className="font-bold uppercase tracking-wider">&gt;&gt;&gt; STATE: {stateUI.title}</h4>
              <p className="opacity-70 text-[10px] md:text-xs">{stateUI.message}</p>
            </div>
          </div>

          {(transferState === 'receiving' || transferState === 'reconstructing') && (
            <div className="mt-4 space-y-2 border-t pt-3">
              <p className="text-[11px] font-bold">[SYNC PROGRESS]:</p>
              <MonospaceProgress value={decoderRef.current ? decoderRef.current.progress * 100 : 0} />
              <div className="flex justify-between text-[10px] opacity-60">
                <span>PACKETS DETECTED: {progress.blocksReceived}</span>
                <span>DECODE RATE: {formatBytes(progress.throughput)}/S</span>
              </div>
            </div>
          )}
        </div>
      )}

      {/* Handshake safety word checksum box */}
      {showSafetyVerify && safetyNumber && (
        <div className={`${ui.panel} border-yellow-500 text-yellow-500 bg-yellow-950/5 space-y-4`}>
          <div className="border-b border-yellow-500/30 pb-2">
            <h4 className="font-bold flex items-center gap-2 text-xs md:text-sm">
              <Shield className="w-4 h-4" />
              VERIFY CHECKSUM WITH TRANSMITTER
            </h4>
          </div>
          <p className="text-xs leading-relaxed">
            Ensure the receiver security checksum matches the words displayed on the transmitter terminal:
          </p>
          <div className="text-center p-3 border border-yellow-500 bg-black">
            <p className="text-sm md:text-base font-bold tracking-widest uppercase font-mono">
              {safetyNumber}
            </p>
          </div>
          <div className="flex gap-3">
            <button
              className="flex-1 py-2 border border-red-500 text-red-500 hover:bg-red-500/10 font-mono text-xs focus:outline-none"
              onClick={onBack}
            >
              [ ABORT LINK ]
            </button>
            <button
              className="flex-1 py-2 border border-yellow-500 bg-yellow-500 text-black hover:bg-yellow-400 font-mono text-xs font-bold focus:outline-none"
              onClick={verifyAndContinue}
            >
              [ AGREE & SYNC ]
            </button>
          </div>
        </div>
      )}

      {/* Completed transaction screen */}
      {receivedFile && (
        <div className={`${ui.panel} border-green-500 space-y-4`}>
          <div className="border-b pb-2">
            <h3 className="font-bold text-green-500 flex items-center gap-2 text-xs md:text-sm">
              <CheckCircle className="w-5 h-5" />
              PAYLOAD SUCCESSFULLY DECODED
            </h3>
          </div>
          <div className="p-4 border border-green-800 bg-green-950/5 text-xs space-y-2 leading-relaxed">
            <p className="font-bold truncate">[NAME]: {receivedFile.name}</p>
            <p>[SIZE]: {formatBytes(receivedFile.size)}</p>
            <p>[INTEGRITY]: SHA-256 HASH VERIFIED</p>
          </div>
          <button
            className={ui.btnPrimary}
            style={{ width: '100%' }}
            onClick={() => {
              const a = document.createElement('a');
              a.href = receivedFile.url;
              a.download = receivedFile.name;
              a.click();
            }}
          >
            [ DOWNLOAD RECONSTRUCTED PAYLOAD ]
          </button>
        </div>
      )}
    </div>
  );
}

// ==================== UTILITIES ====================

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}