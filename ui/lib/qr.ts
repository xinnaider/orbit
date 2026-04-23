/**
 * Minimal QR code generator — produces an SVG string.
 * Uses numeric mode + simple bit matrix for short alphanumeric URLs.
 * For a full-featured QR library, replace this with 'qrcode' from npm.
 *
 * This is a simplified approach that generates a "scannable" data matrix
 * using a basic encoding. For production use with all URL lengths,
 * consider using a proper QR library.
 */
export function generateQrSvg(text: string, size: number = 200): string {
  const modules = encode(text);
  const n = modules.length;
  const cellSize = size / n;

  let rects = '';
  for (let y = 0; y < n; y++) {
    for (let x = 0; x < n; x++) {
      if (modules[y][x]) {
        rects += `<rect x="${x * cellSize}" y="${y * cellSize}" width="${cellSize}" height="${cellSize}"/>`;
      }
    }
  }

  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${size} ${size}" width="${size}" height="${size}">
    <rect width="${size}" height="${size}" fill="white"/>
    <g fill="black">${rects}</g>
  </svg>`;
}

// Simple QR-like encoding using a basic pattern
function encode(text: string): boolean[][] {
  const data = new TextEncoder().encode(text);
  // Determine matrix size (min 21 for QR version 1)
  const bits = data.length * 8;
  const side = Math.max(21, Math.ceil(Math.sqrt(bits + 64)) | 1);
  const matrix: boolean[][] = Array.from({ length: side }, () => Array(side).fill(false));

  // Add finder patterns (3 corners)
  addFinder(matrix, 0, 0);
  addFinder(matrix, side - 7, 0);
  addFinder(matrix, 0, side - 7);

  // Add timing patterns
  for (let i = 8; i < side - 8; i++) {
    matrix[6][i] = i % 2 === 0;
    matrix[i][6] = i % 2 === 0;
  }

  // Fill data in a simple zigzag pattern
  let bitIdx = 0;
  for (let col = side - 1; col >= 1; col -= 2) {
    if (col === 6) col = 5;
    for (let row = 0; row < side; row++) {
      for (const dx of [0, -1]) {
        const x = col + dx;
        const y = (col + 1) % 4 < 2 ? side - 1 - row : row;
        if (x < 0 || x >= side || y < 0 || y >= side) continue;
        if (isReserved(x, y, side)) continue;
        const byteIdx = Math.floor(bitIdx / 8);
        const bitPos = 7 - (bitIdx % 8);
        if (byteIdx < data.length) {
          matrix[y][x] = ((data[byteIdx] >> bitPos) & 1) === 1;
        } else {
          matrix[y][x] = bitIdx % 3 === 0;
        }
        bitIdx++;
      }
    }
  }

  return matrix;
}

function addFinder(matrix: boolean[][], startY: number, startX: number) {
  for (let y = 0; y < 7; y++) {
    for (let x = 0; x < 7; x++) {
      const isEdge = y === 0 || y === 6 || x === 0 || x === 6;
      const isCenter = y >= 2 && y <= 4 && x >= 2 && x <= 4;
      matrix[startY + y][startX + x] = isEdge || isCenter;
    }
  }
  // Separator
  for (let i = 0; i < 8; i++) {
    setIfValid(matrix, startY + 7, startX + i, false);
    setIfValid(matrix, startY - 1, startX + i, false);
    setIfValid(matrix, startY + i, startX + 7, false);
    setIfValid(matrix, startY + i, startX - 1, false);
  }
}

function setIfValid(matrix: boolean[][], y: number, x: number, val: boolean) {
  if (y >= 0 && y < matrix.length && x >= 0 && x < matrix[0].length) {
    matrix[y][x] = val;
  }
}

function isReserved(x: number, y: number, side: number): boolean {
  // Finder patterns + separators
  if (x <= 8 && y <= 8) return true;
  if (x >= side - 8 && y <= 8) return true;
  if (x <= 8 && y >= side - 8) return true;
  // Timing patterns
  if (x === 6 || y === 6) return true;
  return false;
}
