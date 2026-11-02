// The companion app's brand files: Omsi-Hub's own icon (the ring and the bus, unchanged) with an
// orange phone badge in the ring's opening. Built from the brand's own drawing
// (assets/brand/logo.svg) and its font (ui/fonts, Hanken Grotesk). Writes assets/brand/companion/:
//   companion-icon.svg        the mark on the app's dark tile
//   companion-mark.svg        the mark alone, for light backgrounds (dark bus)
//   companion-mark-light.svg  the same for dark backgrounds (light bus)
//   companion-logo.svg        mark, "Omsi-Hub" and "Companion", for light backgrounds
//   companion-logo-light.svg  the same for dark backgrounds
//   png/                      the app's icons (Expo: icon, Android adaptive, splash, favicon),
//                             the logos and avatars, and _overzicht.png with all of them
//
//   node tools/companion-brand.mjs

import { execSync } from 'node:child_process';
import { createRequire } from 'node:module';
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const here = dirname(dirname(fileURLToPath(import.meta.url)));
const out = join(here, 'assets', 'brand', 'companion');

const BLUE = '#2a75f7';
const BLUE_ON_DARK = '#6fa1ff';
const ORANGE = '#F58620';
const INK_DARK = '#201e24';
const INK_LIGHT = '#eef1f5';
const TILE = '#0a142e';

// the bus and the two words, as logo.svg draws them (route, bus, "Omsi-", "Hub")
const paths = [...readFileSync(join(here, 'assets', 'brand', 'logo.svg'), 'utf8').matchAll(/<path d="([^"]+)"/g)].map((m) => m[1]);
const [, BUS, OMSI, HUB] = paths;

const font = (w) => readFileSync(join(here, 'ui', 'fonts', `HankenGrotesk-latin-${w}.woff2`)).toString('base64');
const FONT = `<style>@font-face{font-family:'Hanken Grotesk';font-weight:500;src:url(data:font/woff2;base64,${font(500)}) format('woff2')}</style>`;

// ---- the mark ----

// the ring of icon.svg (centre 334,292, radius 167), at 90 % and a little up and left of the
// tile's middle, so the badge has the lower right corner
const RING = 'M485.35 362.58 A167 167 0 1 0 334 459 L420 459';
const OMSI_AT = 'translate(316 300) scale(.9) translate(-334 -292)';
// the badge in the ring's opening, cut free of the ring by a gap; a phone on it
const BADGE = { x: 494, y: 474, r: 82, gap: 14 };
const PHONE = { x: 467, y: 428, w: 54, h: 92, r: 14, line: 10 };
const SLOT = { x: 484, y: 440, w: 20, h: 6 };

let ids = 0;
// mono: one colour throughout (Android's themed icon), the phone cut out of the badge
function mark({ ring = BLUE, bus = INK_LIGHT, badge = ORANGE, phone = '#fff', mono = false } = {}) {
  const id = `m${ids++}`;
  const { x, y, w, h, r, line } = PHONE;
  const glyph = (c) =>
    `<rect x="${x}" y="${y}" width="${w}" height="${h}" rx="${r}" fill="none" stroke="${c}" stroke-width="${line}"/><rect x="${SLOT.x}" y="${SLOT.y}" width="${SLOT.w}" height="${SLOT.h}" rx="${SLOT.h / 2}" fill="${c}"/>`;
  const gap = `<mask id="${id}"><rect x="-2000" y="-2000" width="5000" height="5000" fill="#fff"/><circle cx="${BADGE.x}" cy="${BADGE.y}" r="${BADGE.r + BADGE.gap}" fill="#000"/></mask>`;
  const cut = mono ? `<mask id="${id}p"><circle cx="${BADGE.x}" cy="${BADGE.y}" r="${BADGE.r}" fill="#fff"/>${glyph('#000')}</mask>` : '';
  return `${gap}${cut}
<g mask="url(#${id})"><g transform="${OMSI_AT}"><path d="${RING}" fill="none" stroke="${ring}" stroke-width="46" stroke-linecap="round"/><path d="${BUS}" fill="${bus}"/></g></g>
<circle cx="${BADGE.x}" cy="${BADGE.y}" r="${BADGE.r}" fill="${badge}"${mono ? ` mask="url(#${id}p)"` : ''}/>${mono ? '' : glyph(phone)}`;
}

const svg = (vb, body, title = 'Omsi-Hub Companion') =>
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${vb}" role="img" aria-label="${title}">
<title>${title}</title>
${body}
</svg>
`;

// the tile framed as icon.svg frames it; the mark alone and the square icons framed round the
// mark's middle (360,342: ring and badge together)
const TILE_VB = '54 42 560 560';
const tile = (rx = 123.2, fill = TILE) => `<rect x="54" y="42" width="560" height="560" rx="${rx}" fill="${fill}"/>`;
const MARK_VB = '125 107 470 470';
const SQUARE_VB = '42.5 24.5 635 635';

// ---- the logo: mark left, "Omsi-Hub" over "Companion" ----

// the words as logo.svg has them (caps 170-340), moved a little right to clear the badge
const LOGO_VB = '120 105 1694 470';
const logoBody = (ink) => `${FONT}${mark({ bus: ink })}
<g transform="translate(34 0)"><path d="${OMSI}" fill="${ink}"/>
<path d="${HUB}" fill="${ORANGE}" stroke="${ORANGE}" stroke-width="7" stroke-linejoin="round"/></g>
<text x="616" y="500" font-family="Hanken Grotesk" font-weight="500" font-size="150" fill="${ink === INK_DARK ? BLUE : BLUE_ON_DARK}">Companion</text>`;
const logo = (ink) => svg(LOGO_VB, logoBody(ink));

// ---- writing them ----

rmSync(out, { recursive: true, force: true });
mkdirSync(join(out, 'png'), { recursive: true });
const files = {
  'companion-icon.svg': svg(TILE_VB, `${tile()}\n${mark()}`),
  'companion-mark.svg': svg(MARK_VB, mark({ bus: INK_DARK })),
  'companion-mark-light.svg': svg(MARK_VB, mark()),
  'companion-logo.svg': logo(INK_DARK),
  'companion-logo-light.svg': logo(INK_LIGHT),
  'LEESMIJ.md': `# Omsi-Hub Companion logo

The companion app's mark: Omsi-Hub's own icon (the blue ring and the bus, unchanged) with an orange
phone badge in the ring's opening. \`_overzicht.png\` shows every file on one sheet.

| File | What it is |
|---|---|
| \`companion-icon.svg\` | The mark on the app's dark tile |
| \`companion-mark.svg\`, \`companion-mark-light.svg\` | The mark alone, for light and for dark backgrounds |
| \`companion-logo.svg\`, \`companion-logo-light.svg\` | Mark, "Omsi-Hub" and "Companion", for light and for dark backgrounds (the font is inside the file) |
| \`png/app/\` | Drop-in replacements for the companion app's \`assets/images/\` (icon, Android adaptive layers, splash, favicon), at the sizes it already uses |
| \`png/\` | The icon, marks, avatars (dark, light, blue) and logos as PNG |

To regenerate them (after a change in the logo, say), run: \`node tools/companion-brand.mjs\`
`,
};
for (const [name, body] of Object.entries(files)) writeFileSync(join(out, name), body);

// PNGs: [file, viewBox, body, width px, height px]. The app's sizes follow what the companion app
// (Omsi-hub-companion-app/assets/images) already has, so they drop in as they are.
const full = (fill) => `<rect x="-2000" y="-2000" width="5000" height="5000" fill="${fill}"/>`;
const PNG = [
  // Expo / iOS: square, no transparency (the system rounds it)
  ['app/icon.png', SQUARE_VB, `${full(TILE)}${mark()}`, 1024, 1024],
  // Android adaptive: the mark inside the safe zone, on its own layer
  ['app/android-icon-foreground.png', '-140 -158 1000 1000', mark(), 1024, 1024],
  ['app/android-icon-background.png', '0 0 10 10', full(TILE), 1024, 1024],
  ['app/android-icon-monochrome.png', '-140 -158 1000 1000', mark({ ring: '#fff', bus: '#fff', badge: '#fff', mono: true }), 1024, 1024],
  ['app/splash-icon.png', SQUARE_VB, mark(), 512, 512],
  ['app/favicon.png', TILE_VB, `${tile(112)}${mark()}`, 96, 96],
  ['companion-icon.png', TILE_VB, `${tile()}${mark()}`, 1024, 1024],
  ['companion-mark.png', MARK_VB, mark({ bus: INK_DARK }), 1024, 1024],
  ['companion-mark-light.png', MARK_VB, mark(), 1024, 1024],
  ['companion-avatar-dark.png', SQUARE_VB, `${full(TILE)}${mark()}`, 1024, 1024],
  ['companion-avatar-light.png', SQUARE_VB, `${full('#fff')}${mark({ bus: INK_DARK })}`, 1024, 1024],
  ['companion-avatar-blue.png', SQUARE_VB, `${full(BLUE)}${mark({ ring: '#fff', bus: '#fff' })}`, 1024, 1024],
  ['companion-logo.png', LOGO_VB, logoBody(INK_DARK), 3388, 940],
  ['companion-logo-light.png', LOGO_VB, logoBody(INK_LIGHT), 3388, 940],
  ['companion-logo-on-dark.png', LOGO_VB, `${full(TILE)}${logoBody(INK_LIGHT)}`, 3388, 940],
  ['companion-logo-on-white.png', LOGO_VB, `${full('#fff')}${logoBody(INK_DARK)}`, 3388, 940],
];

const require = createRequire(import.meta.url);
let playwright;
for (const at of ['playwright', join(execSync('npm root -g').toString().trim(), 'playwright'), join(here, '..', 'Omsi-Hub-Website-admin', 'node_modules', 'playwright')]) {
  try {
    playwright = require(at);
    break;
  } catch {}
}
if (!playwright) throw new Error('Playwright not found (npm i -g playwright)');
let browser;
try {
  browser = await playwright.chromium.launch();
} catch {
  browser = await playwright.chromium.launch({ channel: 'chrome' });
}
const page = await browser.newPage();
mkdirSync(join(out, 'png', 'app'), { recursive: true });
for (const [file, vb, body, w, h] of PNG) {
  await page.setViewportSize({ width: w, height: h });
  await page.setContent(`<!doctype html><html><head><style>html,body{margin:0;background:transparent}svg{display:block}</style></head><body><svg xmlns="http://www.w3.org/2000/svg" viewBox="${vb}" width="${w}" height="${h}">${body}</svg></body></html>`);
  await page.evaluate(async () => {
    await document.fonts.load('500 100px "Hanken Grotesk"');
    await document.fonts.ready;
  });
  await page.screenshot({ path: join(out, 'png', file), omitBackground: true, clip: { x: 0, y: 0, width: w, height: h } });
}

// one sheet with all of them, on a checkerboard
const checker = 'background:#fff url("data:image/svg+xml,%3Csvg xmlns=%27http://www.w3.org/2000/svg%27 width=%2716%27 height=%2716%27%3E%3Crect width=%278%27 height=%278%27 fill=%27%23ddd%27/%3E%3Crect x=%278%27 y=%278%27 width=%278%27 height=%278%27 fill=%27%23ddd%27/%3E%3C/svg%3E")';
const rel = (p) => pathToFileURL(p).href;
const sheet = `<!doctype html><html><head><style>@font-face{font-family:H;src:url(${rel(join(here, 'ui', 'fonts', 'HankenGrotesk-latin-700.woff2'))})}
body{margin:0;padding:40px;background:#0b1224;font-family:H,sans-serif;color:#eef1f5;width:1840px}h1{font-size:40px;margin:0 0 24px}
.g{display:flex;flex-wrap:wrap;gap:14px;align-items:flex-start}.c{${checker};border-radius:10px;padding:12px;display:flex;flex-direction:column;gap:8px}.c img{display:block;height:200px}.c.w img{height:150px}.c span{font-size:13px;color:#2a3140}
.s{display:flex;gap:18px;align-items:end;margin-top:24px}.s img{display:block}</style></head><body>
<h1>Omsi-Hub Companion</h1>
<div class="g">${PNG.filter(([f]) => !f.includes('logo'))
  .map(([f]) => `<div class="c"><img src="${rel(join(out, 'png', f))}"><span>${f}</span></div>`)
  .join('')}</div>
<div class="g" style="margin-top:14px">${PNG.filter(([f]) => f.includes('logo'))
  .map(([f]) => `<div class="c w"><img src="${rel(join(out, 'png', f))}"><span>${f}</span></div>`)
  .join('')}</div>
<div class="s">${[180, 120, 76, 48, 32, 20].map((s) => `<img src="${rel(join(out, 'png', 'companion-icon.png'))}" width="${s}" height="${s}">`).join('')}</div>
</body></html>`;
writeFileSync(join(out, '_sheet.html'), sheet);
await page.setViewportSize({ width: 1920, height: 1080 });
await page.goto(rel(join(out, '_sheet.html')));
await page.evaluate(() => document.fonts.ready);
await page.screenshot({ path: join(out, '_overzicht.png'), fullPage: true });
rmSync(join(out, '_sheet.html'));
await browser.close();
console.log(`companion brand files in ${relative(here, out)}`);
