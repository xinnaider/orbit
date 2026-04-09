import puppeteer from 'puppeteer';
import fs from 'fs';
import path from 'path';
import toIco from 'to-ico';

const svgFull = fs.readFileSync('tauri/icons/orbit-source.svg', 'utf8');
const svgSmall = fs.readFileSync('tauri/icons/orbit-small.svg', 'utf8');
const iconsDir = 'tauri/icons';

// sizes <= 64 use bold simplified design; larger sizes use full detail
function svgFor(size) {
  return size <= 64 ? svgSmall : svgFull;
}

async function renderWithChrome(browser, size) {
  const svg = svgFor(size);
  // Embed SVG in an HTML page at exact pixel size — Chrome renders it perfectly
  const html = `<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<style>
  * { margin: 0; padding: 0; }
  html, body { width: ${size}px; height: ${size}px; background: transparent; overflow: hidden; }
  img { width: ${size}px; height: ${size}px; display: block; }
</style>
</head>
<body>
  <img src="data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}" width="${size}" height="${size}">
</body>
</html>`;

  const page = await browser.newPage();
  await page.setViewport({ width: size, height: size, deviceScaleFactor: 1 });
  await page.setContent(html, { waitUntil: 'networkidle0' });
  const buf = await page.screenshot({ type: 'png', clip: { x: 0, y: 0, width: size, height: size }, omitBackground: true });
  await page.close();
  return buf;
}

const pngSizes = [
  { file: '32x32.png', size: 32 },
  { file: '64x64.png', size: 64 },
  { file: '128x128.png', size: 128 },
  { file: '128x128@2x.png', size: 256 },
  { file: 'icon.png', size: 512 },
];

const squareSizes = [
  { file: 'Square30x30Logo.png', size: 30 },
  { file: 'Square44x44Logo.png', size: 44 },
  { file: 'Square71x71Logo.png', size: 71 },
  { file: 'Square89x89Logo.png', size: 89 },
  { file: 'Square107x107Logo.png', size: 107 },
  { file: 'Square142x142Logo.png', size: 142 },
  { file: 'Square150x150Logo.png', size: 150 },
  { file: 'Square284x284Logo.png', size: 284 },
  { file: 'Square310x310Logo.png', size: 310 },
  { file: 'StoreLogo.png', size: 50 },
];

const iosSizes = [
  { file: 'ios/AppIcon-20x20@1x.png', size: 20 },
  { file: 'ios/AppIcon-20x20@2x.png', size: 40 },
  { file: 'ios/AppIcon-20x20@2x-1.png', size: 40 },
  { file: 'ios/AppIcon-20x20@3x.png', size: 60 },
  { file: 'ios/AppIcon-29x29@1x.png', size: 29 },
  { file: 'ios/AppIcon-29x29@2x.png', size: 58 },
  { file: 'ios/AppIcon-29x29@2x-1.png', size: 58 },
  { file: 'ios/AppIcon-29x29@3x.png', size: 87 },
  { file: 'ios/AppIcon-40x40@1x.png', size: 40 },
  { file: 'ios/AppIcon-40x40@2x.png', size: 80 },
  { file: 'ios/AppIcon-40x40@2x-1.png', size: 80 },
  { file: 'ios/AppIcon-40x40@3x.png', size: 120 },
  { file: 'ios/AppIcon-60x60@2x.png', size: 120 },
  { file: 'ios/AppIcon-60x60@3x.png', size: 180 },
  { file: 'ios/AppIcon-76x76@1x.png', size: 76 },
  { file: 'ios/AppIcon-76x76@2x.png', size: 152 },
  { file: 'ios/AppIcon-83.5x83.5@2x.png', size: 167 },
  { file: 'ios/AppIcon-512@2x.png', size: 1024 },
];

const androidSizes = [
  { file: 'android/mipmap-mdpi/ic_launcher_foreground.png', size: 108 },
  { file: 'android/mipmap-hdpi/ic_launcher_foreground.png', size: 162 },
  { file: 'android/mipmap-xhdpi/ic_launcher_foreground.png', size: 216 },
  { file: 'android/mipmap-xxhdpi/ic_launcher_foreground.png', size: 324 },
  { file: 'android/mipmap-xxxhdpi/ic_launcher_foreground.png', size: 432 },
];

const allSizes = [...pngSizes, ...squareSizes, ...iosSizes, ...androidSizes];

async function main() {
  console.log('Launching Chrome...');
  const browser = await puppeteer.launch({ headless: true, args: ['--no-sandbox'] });

  for (const { file, size } of allSizes) {
    const outPath = path.join(iconsDir, file);
    const buf = await renderWithChrome(browser, size);
    fs.writeFileSync(outPath, buf);
    console.log(`  ✓ ${file} (${size}x${size})`);
  }

  // ICO: embed 16,24,32,48,64,128,256 — small sizes use simplified SVG
  const icoSizes = [16, 24, 32, 48, 64, 128, 256];
  const icoBufs = await Promise.all(icoSizes.map(s => renderWithChrome(browser, s)));
  const ico = await toIco(icoBufs, { sizes: icoSizes });
  fs.writeFileSync(path.join(iconsDir, 'icon.ico'), ico);
  console.log(`  ✓ icon.ico (${icoSizes.join(',')})`);

  await browser.close();
  console.log('\nAll icons generated with Chrome renderer.');
}

main().catch(console.error);
