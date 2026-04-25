import QRCode from 'qrcode';

export async function generateQrSvg(text: string, size: number = 200): Promise<string> {
  return QRCode.toString(text, {
    type: 'svg',
    width: size,
    margin: 2,
    color: {
      dark: '#000000',
      light: '#ffffff',
    },
  });
}
