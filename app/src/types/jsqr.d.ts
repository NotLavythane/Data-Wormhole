declare module 'jsqr' {
  interface QRCode {
    data: string;
    chunks: Array<{
      type: string;
      bytes: number[];
    }>;
    location: {
      topLeftCorner: { x: number; y: number };
      topRightCorner: { x: number; y: number };
      bottomLeftCorner: { x: number; y: number };
      bottomRightCorner: { x: number; y: number };
      topLeftFinderPattern: { x: number; y: number };
      topRightFinderPattern: { x: number; y: number };
      bottomLeftFinderPattern: { x: number; y: number };
      bottomRightAlignmentPattern?: { x: number; y: number };
    };
  }

  function jsQR(
    data: Uint8ClampedArray,
    width: number,
    height: number,
    options?: {
      inversionAttempts?: 'dontInvert' | 'onlyInvert' | 'attemptBoth';
    }
  ): QRCode | null;

  export default jsQR;
}