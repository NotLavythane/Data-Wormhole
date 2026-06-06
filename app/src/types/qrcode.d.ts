declare module 'qrcode' {
  export function toDataURL(
    text: string | Buffer,
    options?: {
      errorCorrectionLevel?: 'L' | 'M' | 'Q' | 'H';
      type?: string;
      quality?: number;
      margin?: number;
      color?: {
        dark?: string;
        light?: string;
      };
      width?: number;
    }
  ): Promise<string>;

  export function toCanvas(
    canvas: HTMLCanvasElement,
    text: string | Buffer,
    options?: {
      errorCorrectionLevel?: 'L' | 'M' | 'Q' | 'H';
      margin?: number;
      width?: number;
      color?: {
        dark?: string;
        light?: string;
      };
    }
  ): Promise<void>;
}