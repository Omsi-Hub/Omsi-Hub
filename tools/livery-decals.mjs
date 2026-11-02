// Decals for an Omsi-Hub livery, from the brand's own drawing (assets/brand/logo.svg) and its font
// (ui/fonts, Hanken Grotesk). Writes assets/brand/livery/:
//   <group>/<name>.png      transparent, the long side up to 4096 px (the livery studio's image
//                           layer takes them as they are; a paint program as well)
//   svg/<group>/<name>.svg  the same drawing, its font inside
//   _overzicht.png          all of them on one sheet
//   _voorbeeld.png          two buses wearing them
//
//   node tools/livery-decals.mjs
//
// Needs Playwright (its Chromium, or Chrome itself).

import { execSync } from 'node:child_process';
import { createRequire } from 'node:module';
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const here = dirname(dirname(fileURLToPath(import.meta.url)));
const brand = join(here, 'assets', 'brand');
const out = join(brand, 'livery');

const BLUE = '#2a75f7';
const ORANGE = '#F58620';
const INK = '#201e24';
const LIGHT = '#eef1f5';
const NAVY = '#0a142e';
const NAVY2 = '#14295e';
const RED = '#E43334';
const GREEN = '#0F7E3A';
const YELLOW = '#FFC72C';
const WHITE = '#ffffff';

// ---- the brand's drawing, taken apart ----

const logoSrc = readFileSync(join(brand, 'logo.svg'), 'utf8');
const ROUTE = /<path d="([^"]+)" fill="none" stroke="#2a75f7"/.exec(logoSrc)[1];
const [BUS, WORDS0] = [...logoSrc.matchAll(/<path d="([^"]+)" fill="#201e24"\/>/g)].map((m) => m[1]);
const WORDS1 = /<path d="([^"]+)" fill="#F58620"/.exec(logoSrc)[1];
const STOPS = logoSrc.slice(logoSrc.indexOf('<circle cx="428"'), logoSrc.lastIndexOf('</svg>')).trim();
const g1 = STOPS.indexOf('<g transform');
const g2 = STOPS.indexOf('<g transform', g1 + 5);
// the three stop signs (U-Bahn, Haltestelle, Bus), each with the x of its middle in the logo
const STOP = { u: [STOPS.slice(0, g1), 428], h: [STOPS.slice(g1, g2), 1050.7], bus: [STOPS.slice(g2), 1673.4] };
const STOP_X = [428, 1050.7, 1673.4];
const stop = (k, x, y, s = 1) => `<g transform="translate(${x} ${y}) scale(${s}) translate(${-STOP[k][1]} -459)">${STOP[k][0]}</g>`;
// (a stop in one colour: a ring, the line cut open around it)
const ringStop = (x, y, c) => `<circle cx="${x}" cy="${y}" r="44" fill="none" stroke="${c}" stroke-width="20"/>`;
const cutStops = (id, xs, y, w, h) => `<mask id="${id}" maskUnits="userSpaceOnUse" x="-100" y="-100" width="${w + 200}" height="${h + 200}"><rect x="-100" y="-100" width="${w + 200}" height="${h + 200}" fill="#fff"/>${xs.map((x) => `<circle cx="${x}" cy="${y}" r="64" fill="#000"/>`).join('')}</mask>`;

// the ring round the bus: its middle (334 292), 167 across, the line leaving it at the foot
const RING = 'M485.35 362.58 A167 167 0 1 0 334 459 L420 459';
const MARK_VB = '124 82 420 420';
const mark = (ring, bus) => `<path d="${RING}" fill="none" stroke="${ring}" stroke-width="46" stroke-linecap="round"/><path d="${BUS}" fill="${bus}"/>`;
const words = (ink, hub = ORANGE) => `<path d="${WORDS0}" fill="${ink}"/><path d="${WORDS1}" fill="${hub}" stroke="${hub}" stroke-width="7" stroke-linejoin="round"/>`;

const LOGO_VB = '96 54 1702.1 516';
const logoColour = (ink) => `<path d="${ROUTE}" fill="none" stroke="${BLUE}" stroke-width="46" stroke-linecap="round"/><path d="${BUS}" fill="${ink}"/>${words(ink)}${STOPS}`;
const logoMono = (c) =>
  `${cutStops('cut', STOP_X, 459, 2000, 700)}<path d="${ROUTE}" fill="none" stroke="${c}" stroke-width="46" stroke-linecap="round" mask="url(#cut)"/><path d="${BUS}" fill="${c}"/>${words(c, c)}${STOP_X.map((x) => ringStop(x, 459, c)).join('')}`;

// ---- the font ----

const font = (w) => readFileSync(join(here, 'ui', 'fonts', `HankenGrotesk-latin-${w}.woff2`)).toString('base64');
const FONTS = [500, 700, 800].map((w) => `@font-face{font-family:'Hanken Grotesk';font-weight:${w};src:url(data:font/woff2;base64,${font(w)}) format('woff2')}`).join('');
const T = (x, y, size, weight, fill, text, extra = '') =>
  `<text x="${x}" y="${y}" font-family="Hanken Grotesk" font-size="${size}" font-weight="${weight}" fill="${fill}"${extra}>${text.replace(/& /g, '&amp; ')}</text>`;

// ---- the decals ----
// each: group, name, viewBox, body; px (its width) or pxh (its height); fit: the viewBox shrunk to
// the drawing (that much air round it); grow: the width grown to the text #txt (stickers, plates)

const D = [];
const add = (group, name, o) => D.push({ group, name, ...o });

// 01 logo
add('01-logo', 'logo-op-licht', { vb: LOGO_VB, px: 4096, body: logoColour(INK) });
add('01-logo', 'logo-op-donker', { vb: LOGO_VB, px: 4096, body: logoColour(LIGHT) });
add('01-logo', 'logo-wit', { vb: LOGO_VB, px: 4096, body: logoMono(WHITE) });
add('01-logo', 'logo-zwart', { vb: LOGO_VB, px: 4096, body: logoMono(INK) });
add('01-logo', 'logo-blauw', { vb: LOGO_VB, px: 4096, body: logoMono(BLUE) });
add('01-logo', 'woordmerk-op-licht', { vb: '566 154 1196 206', px: 4096, body: words(INK) });
add('01-logo', 'woordmerk-op-donker', { vb: '566 154 1196 206', px: 4096, body: words(LIGHT) });
add('01-logo', 'woordmerk-wit', { vb: '566 154 1196 206', px: 4096, body: words(WHITE, WHITE) });
add('01-logo', 'beeldmerk-op-licht', { vb: MARK_VB, px: 2048, body: mark(BLUE, INK) });
add('01-logo', 'beeldmerk-op-donker', { vb: MARK_VB, px: 2048, body: mark(BLUE, LIGHT) });
add('01-logo', 'beeldmerk-wit', { vb: MARK_VB, px: 2048, body: mark(WHITE, WHITE) });
add('01-logo', 'beeldmerk-blauw', { vb: MARK_VB, px: 2048, body: mark(BLUE, BLUE) });
const badge = (rim, tile, bus) =>
  `<circle cx="500" cy="500" r="496" fill="${rim}"/><circle cx="500" cy="500" r="462" fill="${tile}"/><g transform="translate(500 492) scale(1.5) translate(-334 -304)">${mark(BLUE, bus)}</g>`;
add('01-logo', 'embleem-rond-donker', { vb: '0 0 1000 1000', px: 2048, body: badge(WHITE, NAVY, LIGHT) });
add('01-logo', 'embleem-rond-licht', { vb: '0 0 1000 1000', px: 2048, body: badge(BLUE, WHITE, INK) });
add('01-logo', 'app-icoon', { vb: '0 0 1000 1000', px: 2048, body: `<rect width="1000" height="1000" rx="220" fill="${NAVY}"/><g transform="translate(500 492) scale(1.62) translate(-334 -304)">${mark(BLUE, LIGHT)}</g>` });

// 02 lines: the route line with its stops, the brand's own stripe
const LINE_STOPS = [['u', 300], ['h', 900], ['bus', 1500], ['u', 2100], ['h', 2700]];
const routeLine = (c, stops) => {
  const line = `<path d="M40 70 H2960" stroke="${c}" stroke-width="46" stroke-linecap="round"/>`;
  if (stops === 'none') return line;
  if (stops === 'colour') return line + LINE_STOPS.map(([k, x]) => stop(k, x, 70)).join('');
  const xs = LINE_STOPS.map(([, x]) => x);
  return `${cutStops('cut', xs, 70, 3000, 140)}<g mask="url(#cut)">${line}</g>${xs.map((x) => ringStop(x, 70, c)).join('')}`;
};
add('02-lijnen', 'routelijn-haltes', { vb: '0 0 3000 140', px: 4096, body: routeLine(BLUE, 'colour') });
add('02-lijnen', 'routelijn-haltes-wit', { vb: '0 0 3000 140', px: 4096, body: routeLine(WHITE, 'mono') });
add('02-lijnen', 'routelijn-haltes-blauw', { vb: '0 0 3000 140', px: 4096, body: routeLine(BLUE, 'mono') });
add('02-lijnen', 'routelijn-kaal-blauw', { vb: '0 0 3000 140', px: 4096, body: routeLine(BLUE, 'none') });
add('02-lijnen', 'routelijn-kaal-oranje', { vb: '0 0 3000 140', px: 4096, body: routeLine(ORANGE, 'none') });
// the ring with the line leaving it, as the logo starts: for the front or the back of a bus
const RR = [['u', 428], ['h', 1150], ['bus', 1870], ['u', 2590]];
const ringRoute = (c, flip, mono) => {
  const vbx = 120;
  const vbw = 2630;
  const at = (x) => (flip ? 2 * vbx + vbw - x : x);
  const path = `<path d="M485.35 362.58 A167 167 0 1 0 334 459 L2700 459" fill="none" stroke="${c}" stroke-width="46" stroke-linecap="round"${flip ? ` transform="translate(${2 * vbx + vbw} 0) scale(-1 1)"` : ''}/>`;
  if (!mono) return path + RR.map(([k, x]) => stop(k, at(x), 459)).join('');
  const xs = RR.map(([, x]) => at(x));
  return `${cutStops('cut', xs, 459, 3000, 700)}<g mask="url(#cut)">${path}</g>${xs.map((x) => ringStop(x, 459, c)).join('')}`;
};
add('02-lijnen', 'ring-routelijn-rechts', { vb: '120 80 2630 450', px: 4096, body: ringRoute(BLUE, false, false) });
add('02-lijnen', 'ring-routelijn-links', { vb: '120 80 2630 450', px: 4096, body: ringRoute(BLUE, true, false) });
add('02-lijnen', 'ring-routelijn-rechts-wit', { vb: '120 80 2630 450', px: 4096, body: ringRoute(WHITE, false, true) });
add('02-lijnen', 'ring-routelijn-links-wit', { vb: '120 80 2630 450', px: 4096, body: ringRoute(WHITE, true, true) });
for (const k of ['u', 'h', 'bus']) add('02-lijnen', `halte-${k}`, { vb: '-66 -66 132 132', px: 1024, body: stop(k, 0, 0) });

// 03 sweeps: a band rising to the back, with lines along its edge at an even distance
const EDGE = 'M-20 1000 L1400 1000 C2400 1000 2900 150 4020 150';
const AREA = `${EDGE} L4020 1220 L-20 1220 Z`;
const band = (id, d1, d2, c) =>
  `<mask id="${id}" maskUnits="userSpaceOnUse" x="-50" y="-400" width="4100" height="1700"><path d="${EDGE}" fill="none" stroke="#fff" stroke-width="${2 * d2}"/><path d="${EDGE}" fill="none" stroke="#000" stroke-width="${2 * d1}"/><path d="${AREA}" fill="#000"/></mask><rect x="-50" y="-400" width="4100" height="1700" fill="${c}" mask="url(#${id})"/>`;
const sweep = (main, b1, b2) => `${band('b1', 40, 100, b1)}${band('b2', 135, 155, b2)}<path d="${AREA}" fill="${main}"/>`;
add('03-swoosh', 'swoosh-blauw-oranje', { vb: '0 0 4000 1200', px: 4096, body: sweep(BLUE, ORANGE, BLUE) });
add('03-swoosh', 'swoosh-navy-blauw', { vb: '0 0 4000 1200', px: 4096, body: sweep(NAVY, BLUE, ORANGE) });
add('03-swoosh', 'swoosh-oranje-blauw', { vb: '0 0 4000 1200', px: 4096, body: sweep(ORANGE, BLUE, ORANGE) });
add('03-swoosh', 'swoosh-wit', { vb: '0 0 4000 1200', px: 4096, body: sweep(WHITE, WHITE, WHITE) });
// dots that fade out: a grid of rows set off by half, smaller to the right and the top
const halftone = (c) => {
  const dots = [];
  const step = 48;
  for (let row = 0, y = step / 2; y < 1000; row++, y += step * 0.866) {
    for (let x = row % 2 ? step : step / 2; x < 4000; x += step) {
      const t = Math.min(1, (x + 0.6 * (1000 - y)) / 4200);
      const r = 22 * (1 - t) ** 1.35;
      if (r > 2.2) dots.push(`<circle cx="${x.toFixed(1)}" cy="${y.toFixed(1)}" r="${r.toFixed(2)}"/>`);
    }
  }
  return `<g fill="${c}">${dots.join('')}</g>`;
};
for (const [n, c] of [['blauw', BLUE], ['oranje', ORANGE], ['navy', NAVY], ['wit', WHITE]]) add('03-swoosh', `stippen-verloop-${n}`, { vb: '0 0 4000 1000', px: 4096, body: halftone(c) });
const fade = (c) => `<defs><linearGradient id="f"><stop offset="0" stop-color="${c}"/><stop offset=".15" stop-color="${c}"/><stop offset="1" stop-color="${c}" stop-opacity="0"/></linearGradient></defs><rect width="4000" height="600" fill="url(#f)"/>`;
for (const [n, c] of [['blauw', BLUE], ['oranje', ORANGE], ['navy', NAVY]]) add('03-swoosh', `kleurverloop-${n}`, { vb: '0 0 4000 600', px: 4096, body: fade(c) });
const pin = (a, b) => `<rect y="10" width="4000" height="40" fill="${a}"/><rect y="74" width="4000" height="16" fill="${b}"/>`;
add('03-swoosh', 'biesjes-blauw-oranje', { vb: '0 0 4000 100', px: 4096, body: pin(BLUE, ORANGE) });
add('03-swoosh', 'biesjes-wit-oranje', { vb: '0 0 4000 100', px: 4096, body: pin(WHITE, ORANGE) });
add('03-swoosh', 'biesjes-navy-blauw', { vb: '0 0 4000 100', px: 4096, body: pin(NAVY, BLUE) });

// 04 fleet numbers: each digit in the same box (tabular), so they stand together in any order
const digit = (d, c) => T(350, 860, 1000, 800, c, d, ' text-anchor="middle" style="font-variant-numeric:tabular-nums"');
for (const [n, c] of [['wit', WHITE], ['zwart', INK], ['navy', NAVY]]) {
  for (let d = 0; d <= 9; d++) add('04-wagennummers', `${n}/cijfer-${d}`, { vb: '0 0 700 1000', pxh: 1024, body: digit(String(d), c) });
}
const plate = (label, number) =>
  `<rect data-fitw="1" x="0" y="0" width="1600" height="560" rx="64" fill="${NAVY}"/><g transform="translate(280 280) scale(1.12) translate(-334 -292)">${mark(BLUE, LIGHT)}</g><path d="M560 120 V440" stroke="#2b3d6b" stroke-width="6"/><g id="txt">${label ? T(640, 190, 64, 700, '#8ea3cc', label, ' letter-spacing="14"') : ''}${number ? T(632, label ? 450 : 400, label ? 300 : 320, 800, WHITE, number) : ''}</g>`;
add('04-wagennummers', 'wagenbord-1024', { vb: '0 0 1600 560', px: 2048, grow: 80, body: plate('WAGEN', '1024') });
add('04-wagennummers', 'wagenbord-leeg-met-label', { vb: '0 0 1600 560', px: 2048, body: plate('WAGEN', '') });
add('04-wagennummers', 'wagenbord-leeg', { vb: '0 0 1600 560', px: 2048, body: plate('', '') });

// 05 words: the website, the slogan
const url = (ring, bus, ink) => `<g id="c"><g transform="translate(150 150) scale(.62) translate(-334 -292)">${mark(ring, bus)}</g>${T(310, 218, 190, 700, ink, 'omsihub.com')}</g>`;
add('05-teksten', 'website-op-licht', { vb: '0 0 2000 300', px: 4096, fit: 30, body: url(BLUE, INK, INK) });
add('05-teksten', 'website-op-donker', { vb: '0 0 2000 300', px: 4096, fit: 30, body: url(BLUE, LIGHT, LIGHT) });
add('05-teksten', 'website-wit', { vb: '0 0 2000 300', px: 4096, fit: 30, body: url(WHITE, WHITE, WHITE) });
const SLOGAN = {
  en: ['Built for realism.', 'Made for immersion.', 'The add-on for openOMSI'],
  de: ['Gebaut für Realismus.', 'Gemacht fürs Eintauchen.', 'Das Add-on für openOMSI'],
  nl: ['Gebouwd voor realisme.', 'Gemaakt voor beleving.', 'De add-on voor openOMSI'],
};
for (const [lang, [a, b]] of Object.entries(SLOGAN)) {
  const line = (ink) => `<g id="c">${T(0, 200, 200, 800, ink, `${a} <tspan fill="${ORANGE}">${b}</tspan>`)}</g>`;
  const stack = (ink) => `<g id="c">${T(0, 200, 200, 800, ink, a)}${T(0, 440, 200, 800, ORANGE, b)}</g>`;
  add('05-teksten', `slogan-${lang}-op-licht`, { vb: '0 0 5000 300', px: 4096, fit: 30, body: line(INK) });
  add('05-teksten', `slogan-${lang}-op-donker`, { vb: '0 0 5000 300', px: 4096, fit: 30, body: line(WHITE) });
  add('05-teksten', `slogan-${lang}-twee-regels-op-licht`, { vb: '0 0 3000 500', px: 4096, fit: 30, body: stack(INK) });
  add('05-teksten', `slogan-${lang}-twee-regels-op-donker`, { vb: '0 0 3000 500', px: 4096, fit: 30, body: stack(WHITE) });
}

// 06 advertising panels: the side's frame (3 : 1) and the back (2 : 1)
const ad = (lang, dark) => {
  const [a, b, c] = SLOGAN[lang];
  const ink = dark ? WHITE : INK;
  const soft = dark ? '#9fb0d0' : '#5b6478';
  const bg = dark
    ? `<defs><linearGradient id="bg" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="${NAVY}"/><stop offset="1" stop-color="${NAVY2}"/></linearGradient><clipPath id="frame"><rect width="3000" height="1000"/></clipPath></defs><rect width="3000" height="1000" fill="url(#bg)"/>`
    : `<defs><clipPath id="frame"><rect width="3000" height="1000"/></clipPath></defs><rect width="3000" height="1000" fill="${WHITE}"/>`;
  return `${bg}<g clip-path="url(#frame)"><circle cx="2780" cy="420" r="560" fill="none" stroke="${BLUE}" stroke-opacity="${dark ? 0.22 : 0.12}" stroke-width="120"/><g transform="translate(1650 380) scale(.55)" opacity="${dark ? 0.55 : 0.4}">${halftone(BLUE)}</g></g>
<g transform="translate(40 30) scale(1.17) translate(-96 -54)">${dark ? logoColour(LIGHT) : logoColour(INK)}</g>
${T(100, 800, 112, 800, ink, `${a} <tspan fill="${ORANGE}">${b}</tspan>`)}
${T(102, 910, 64, 500, soft, `${c}  ·  <tspan font-weight="700" fill="${ink}">omsihub.com</tspan>`)}`;
};
for (const lang of Object.keys(SLOGAN)) {
  add('06-reclame', `zijpaneel-${lang}-donker`, { vb: '0 0 3000 1000', px: 4096, body: ad(lang, true) });
  add('06-reclame', `zijpaneel-${lang}-licht`, { vb: '0 0 3000 1000', px: 4096, body: ad(lang, false) });
}
const rear = (dark) => {
  const ink = dark ? LIGHT : INK;
  return `<rect width="2000" height="1000" rx="0" fill="${dark ? NAVY : WHITE}"/><g transform="translate(470 500) scale(2.05) translate(-334 -300)">${mark(BLUE, ink)}</g><g transform="translate(900 345) scale(.92) translate(-582 -170)">${words(ink)}</g>${T(905, 660, 104, 700, dark ? '#9fb0d0' : '#5b6478', 'omsihub.com')}<path d="M905 735 H1840" stroke="${BLUE}" stroke-width="22" stroke-linecap="round"/>${stop('h', 1450, 735, 0.36)}${stop('bus', 1840, 735, 0.36)}${stop('u', 1060, 735, 0.36)}`;
};
add('06-reclame', 'achterkant-donker', { vb: '0 0 2000 1000', px: 4096, body: rear(true) });
add('06-reclame', 'achterkant-licht', { vb: '0 0 2000 1000', px: 4096, body: rear(false) });

// 07 pictograms: drawn in a 1000 box, in `fg` on `bg` (some cut a gap in `bg`)
const ICON = {
  rolstoel: (fg, bg) => `<g fill="none" stroke-linecap="round" stroke-linejoin="round">
<path d="M690 735 A205 205 0 1 1 449.4 533.1" stroke="${fg}" stroke-width="64"/>
<path d="M436 350 L458 600 L690 600 L776 842" stroke="${bg}" stroke-width="150"/>
<path d="M436 350 L458 600 L690 600 L776 842" stroke="${fg}" stroke-width="86"/>
<path d="M446 452 L620 452" stroke="${fg}" stroke-width="70"/></g><circle cx="430" cy="218" r="80" fill="${fg}"/>`,
  kinderwagen: (fg) => `<g fill="${fg}"><path d="M200 470 H760 A280 230 0 0 1 200 470 Z"/><path d="M200 436 A262 262 0 0 1 462 174 V436 Z"/><circle cx="350" cy="812" r="74"/><circle cx="640" cy="812" r="74"/></g><path d="M760 470 L824 300 H902" fill="none" stroke="${fg}" stroke-width="50" stroke-linecap="round" stroke-linejoin="round"/>`,
  sneeuw: (fg) => {
    let p = '';
    for (let k = 0; k < 6; k++) {
      const a = (Math.PI / 3) * k - Math.PI / 2;
      const P = (r, da = 0) => [500 + r * Math.cos(a + da), 500 + r * Math.sin(a + da)];
      const [ex, ey] = P(340);
      const [bx, by] = P(210);
      const tw = (s) => {
        const b = a + s * 0.85;
        return `M${bx.toFixed(1)} ${by.toFixed(1)} L${(bx + 120 * Math.cos(b)).toFixed(1)} ${(by + 120 * Math.sin(b)).toFixed(1)}`;
      };
      p += `M500 500 L${ex.toFixed(1)} ${ey.toFixed(1)} ${tw(1)} ${tw(-1)} `;
    }
    return `<path d="${p}" fill="none" stroke="${fg}" stroke-width="58" stroke-linecap="round"/>`;
  },
  camera: (fg, bg) => `<g transform="rotate(-14 470 470)" fill="${fg}"><rect x="210" y="360" width="460" height="220" rx="40"/><rect x="190" y="320" width="480" height="56" rx="22"/><rect x="112" y="410" width="120" height="120" rx="20"/><circle cx="330" cy="470" r="34" fill="${bg}"/></g><path d="M600 580 L700 690 H800" fill="none" stroke="${fg}" stroke-width="56" stroke-linecap="round" stroke-linejoin="round"/><rect x="790" y="560" width="64" height="290" rx="24" fill="${fg}"/>`,
  kaartje: (fg, bg) => `<g transform="rotate(-10 500 500)"><rect x="160" y="290" width="680" height="420" rx="44" fill="${fg}"/><circle cx="160" cy="500" r="64" fill="${bg}"/><circle cx="840" cy="500" r="64" fill="${bg}"/><path d="M650 320 V680" stroke="${bg}" stroke-width="18" stroke-dasharray="34 26"/><rect x="270" y="400" width="300" height="44" rx="22" fill="${bg}"/><rect x="270" y="478" width="230" height="44" rx="22" fill="${bg}"/><rect x="270" y="556" width="270" height="44" rx="22" fill="${bg}"/></g>`,
  instappen: (fg) => `<path d="M560 860 V170 H860 V860" fill="none" stroke="${fg}" stroke-width="56" stroke-linejoin="round"/><path d="M120 520 H540" stroke="${fg}" stroke-width="90" stroke-linecap="round"/><path d="M500 360 L720 520 L500 680 Z" fill="${fg}" stroke="${fg}" stroke-width="24" stroke-linejoin="round"/>`,
  uitstappen: (fg) => `<path d="M140 860 V170 H440 V860" fill="none" stroke="${fg}" stroke-width="56" stroke-linejoin="round"/><path d="M300 520 H850" stroke="${fg}" stroke-width="90" stroke-linecap="round"/><path d="M700 360 L900 520 L700 680 Z" fill="${fg}" stroke="${fg}" stroke-width="24" stroke-linejoin="round"/>`,
  verboden: (fg, bg) => `<circle cx="500" cy="500" r="380" fill="${fg}"/><rect x="240" y="438" width="520" height="124" rx="16" fill="${bg}"/>`,
  rookverbod: () => `<rect x="190" y="462" width="500" height="96" fill="${INK}"/><rect x="690" y="462" width="120" height="96" fill="${ORANGE}"/><path d="M770 430 C730 380 810 350 770 290 M820 430 C780 380 860 350 820 290" fill="none" stroke="#7d8594" stroke-width="22" stroke-linecap="round"/><circle cx="500" cy="500" r="370" fill="none" stroke="${RED}" stroke-width="84"/><path d="M238 238 L762 762" stroke="${RED}" stroke-width="84"/>`,
  let_op: () => `<path d="M500 175 L850 790 H150 Z" fill="none" stroke="${INK}" stroke-width="64" stroke-linejoin="round"/><rect x="468" y="390" width="64" height="250" rx="32" fill="${INK}"/><circle cx="500" cy="705" r="40" fill="${INK}"/>`,
  knop: (fg, bg) => `<circle cx="500" cy="500" r="340" fill="${fg}"/><path d="M500 320 V680" stroke="${bg}" stroke-width="30" stroke-linecap="round"/><path d="M440 400 L300 500 L440 600 Z M560 400 L700 500 L560 600 Z" fill="${bg}" stroke="${bg}" stroke-width="16" stroke-linejoin="round"/>`,
};
const P = {
  rolstoel: { icons: ['rolstoel', 'kinderwagen'], bg: BLUE, fg: WHITE, de: ['Rollstuhl & Kinderwagen', 'Bitte hier einsteigen'], nl: ['Rolstoel & kinderwagen', 'Hier instappen'], en: ['Wheelchairs & buggies', 'Please board here'] },
  instappen: { icons: ['instappen'], bg: BLUE, fg: WHITE, de: ['Einstieg'], nl: ['Instappen'], en: ['Entrance'] },
  uitstappen: { icons: ['uitstappen'], bg: NAVY, fg: WHITE, de: ['Ausstieg'], nl: ['Uitstappen'], en: ['Exit'] },
  verboden: { icons: ['verboden'], bg: RED, fg: WHITE, de: ['Kein Einstieg'], nl: ['Niet instappen'], en: ['No entry'] },
  voorin: { icons: ['kaartje'], bg: BLUE, fg: WHITE, de: ['Bitte vorne einsteigen', 'Fahrschein beim Fahrer vorzeigen'], nl: ['Voorin instappen', 'Vervoerbewijs tonen bij de chauffeur'], en: ['Board at the front', 'Show your ticket to the driver'] },
  knop: { icons: ['knop'], bg: GREEN, fg: WHITE, de: ['Taste drücken', 'Tür öffnet'], nl: ['Druk op de knop', 'De deur gaat open'], en: ['Press the button', 'The door opens'] },
  deur: { icons: ['let_op'], bg: YELLOW, fg: INK, de: ['Vorsicht!', 'Tür öffnet und schließt automatisch'], nl: ['Let op!', 'Deur opent en sluit automatisch'], en: ['Caution!', 'Door opens and closes automatically'] },
  camera: { icons: ['camera'], bg: NAVY, fg: WHITE, de: ['Videoüberwachung', 'Dieses Fahrzeug wird videoüberwacht'], nl: ['Camerabewaking', 'Dit voertuig wordt met camera’s bewaakt'], en: ['CCTV', 'This vehicle is monitored by video'] },
  airco: { icons: ['sneeuw'], bg: BLUE, fg: WHITE, de: ['Klimatisiert', 'Bitte Fenster geschlossen halten'], nl: ['Airco aan boord', 'Houd de ramen dicht'], en: ['Air-conditioned', 'Please keep the windows closed'] },
  rookverbod: { icons: ['rookverbod'], bg: WHITE, fg: INK, de: ['Rauchen verboten', 'Auch E-Zigaretten'], nl: ['Niet roken', 'Ook geen e-sigaret'], en: ['No smoking', 'E-cigarettes included'] },
};
const ring = (bg) => (bg === WHITE ? ' stroke="#cfd5de" stroke-width="8"' : '');
for (const [name, ic] of Object.entries(ICON)) {
  const p = Object.values(P).find((q) => q.icons.includes(name));
  add('07-pictogrammen', name.replace('_', '-'), { vb: '0 0 1000 1000', px: 1024, body: `<rect x="4" y="4" width="992" height="992" rx="130" fill="${p.bg}"${ring(p.bg)}/>${ic(p.fg, p.bg)}` });
}
for (const [name, p] of Object.entries(P)) {
  for (const lang of ['de', 'nl', 'en']) {
    const [title, sub] = p[lang];
    const icons = p.icons.map((k, i) => `<g transform="translate(${30 + i * 500} 40) scale(.52)">${ICON[k](p.fg, p.bg)}</g>`).join('');
    const tx = 30 + p.icons.length * 500 + 40;
    const text = sub ? `${T(tx, 290, 150, 800, p.fg, title)}${T(tx + 4, 430, 82, 500, p.fg, sub, ' opacity=".88"')}` : T(tx, 370, 190, 800, p.fg, title);
    add('08-stickers', `${lang}/${name}`, { vb: '0 0 2000 600', pxh: 768, grow: 90, body: `<rect data-fitw="1" x="4" y="4" width="1992" height="592" rx="70" fill="${p.bg}"${ring(p.bg)}/>${icons}<g id="txt">${text}</g>` });
  }
}

// ---- drawing them ----

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
const DPR = 4;
const page = await browser.newPage({ deviceScaleFactor: DPR });

rmSync(out, { recursive: true, force: true });
const made = [];
for (const d of D) {
  const usesText = d.body.includes('<text');
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${d.vb}"${d.fit ? ` data-fit="${d.fit}"` : ''}${d.grow ? ` data-grow="${d.grow}"` : ''}>${usesText ? `<style>${FONTS}</style>` : ''}${d.body}</svg>`;
  await page.setContent(`<!doctype html><html><head><style>html,body{margin:0;background:transparent}svg{display:block}</style></head><body>${svg}</body></html>`);
  const [vbw, vbh, file] = await page.evaluate(async ({ px, pxh }) => {
    await Promise.all([500, 700, 800].map((w) => document.fonts.load(`${w} 100px "Hanken Grotesk"`)));
    await document.fonts.ready;
    const s = document.querySelector('svg');
    const vb = s.viewBox.baseVal;
    if (s.dataset.grow) {
      const b = s.querySelector('#txt').getBBox();
      const w = Math.ceil(b.x + b.width + Number(s.dataset.grow));
      for (const el of s.querySelectorAll('[data-fitw]')) el.setAttribute('width', w - 2 * Number(el.getAttribute('x')));
      s.setAttribute('viewBox', `${vb.x} ${vb.y} ${w} ${vb.height}`);
    }
    if (s.dataset.fit) {
      const b = s.querySelector('#c').getBBox();
      const p = Number(s.dataset.fit);
      s.setAttribute('viewBox', `${b.x - p} ${b.y - p} ${b.width + 2 * p} ${b.height + 2 * p}`);
    }
    delete s.dataset.fit;
    delete s.dataset.grow;
    for (const el of s.querySelectorAll('[data-fitw]')) el.removeAttribute('data-fitw');
    const v = s.viewBox.baseVal;
    const W = px || (pxh * v.width) / v.height;
    const H = (W * v.height) / v.width;
    const file = s.outerHTML;
    s.style.width = `${W / 4}px`;
    s.style.height = `${H / 4}px`;
    return [Math.round(W), Math.round(H), file];
  }, { px: d.px, pxh: d.pxh });
  await page.setViewportSize({ width: Math.ceil(vbw / DPR) + 1, height: Math.ceil(vbh / DPR) + 1 });
  const png = join(out, d.group, `${d.name}.png`);
  const svgFile = join(out, 'svg', d.group, `${d.name}.svg`);
  mkdirSync(dirname(png), { recursive: true });
  mkdirSync(dirname(svgFile), { recursive: true });
  writeFileSync(svgFile, `${file}\n`);
  await page.locator('svg').screenshot({ path: png, omitBackground: true });
  made.push({ ...d, png, w: vbw, h: vbh });
  console.log(`${d.group}/${d.name}.png  ${vbw} x ${vbh}`);
}

// ---- the sheet of all of them, and two buses wearing them ----

const rel = (p) => pathToFileURL(p).href;
const checker = 'background:#d7dbe2 repeating-conic-gradient(#c3c8d1 0 25%,transparent 0 50%) 0 0/24px 24px';
const groups = [...new Set(made.map((m) => m.group))];
const sheet = `<!doctype html><html><head><style>@font-face{font-family:H;src:url(${rel(join(here, 'ui', 'fonts', 'HankenGrotesk-latin-700.woff2'))})}
body{margin:0;padding:40px;background:#0b1224;font-family:H,sans-serif;color:#eef1f5;width:2320px}h1{font-size:44px;margin:0 0 6px}h2{font-size:26px;margin:36px 0 14px;color:#8ea3cc}
.g{display:flex;flex-wrap:wrap;gap:14px}.c{${checker};border-radius:10px;padding:12px;display:flex;flex-direction:column;gap:8px}.c img{display:block;max-height:150px;max-width:540px;object-fit:contain}.c span{font-size:13px;color:#2a3140}</style></head><body>
<h1>Omsi-Hub livery decals</h1><div style="color:#8ea3cc;font-size:18px">${made.length} decals · transparent PNG + SVG</div>
${groups.map((g) => `<h2>${g}</h2><div class="g">${made.filter((m) => m.group === g && !/^(zwart|navy)\/cijfer/.test(m.name)).map((m) => `<div class="c"><img src="${rel(m.png)}"><span>${m.name}  ·  ${m.w}×${m.h}</span></div>`).join('')}</div>`).join('')}
</body></html>`;
// (as files: pages opened from a file may show the other files)
const tmp = join(out, '.sheet.html');
const view = await browser.newPage({ deviceScaleFactor: 1 });
await view.setViewportSize({ width: 2400, height: 1000 });
writeFileSync(tmp, sheet);
await view.goto(rel(tmp));
await view.evaluate(() => document.fonts.ready);
// (`h` left out: as tall as the picture's own shape makes it)
const img = (name, x, y, w, h, extra = '') => {
  const m = made.find((q) => `${q.group}/${q.name}` === name);
  return `<image href="${rel(m.png)}" x="${x}" y="${y}" width="${w}" height="${h ?? (w * m.h) / m.w}" preserveAspectRatio="none"${extra}/>`;
};
const busSide = (paint, decals, glass = '#1b2433') => {
  const body = 'M120 1080 V250 Q120 150 230 150 H3760 Q3880 150 3890 260 L3900 1080 Z';
  const win = [];
  for (const x of [300, 1240, 1660, 2080, 2500, 2920]) win.push(`<rect x="${x}" y="250" width="${x === 300 ? 340 : 390}" height="380" rx="22" fill="${glass}"/>`);
  return `<defs><clipPath id="body${paint.slice(1)}"><path d="${body}"/></clipPath></defs><path d="${body}" fill="${paint}"/>
<g clip-path="url(#body${paint.slice(1)})">${decals.under}</g>
${win.join('')}<rect x="3480" y="230" width="380" height="520" rx="40" fill="${glass}"/>
<rect x="700" y="250" width="440" height="820" rx="16" fill="#2a3445"/><rect x="722" y="272" width="186" height="640" rx="10" fill="${glass}"/><rect x="932" y="272" width="186" height="640" rx="10" fill="${glass}"/>
<rect x="3010" y="250" width="440" height="820" rx="16" fill="#2a3445"/><rect x="3032" y="272" width="186" height="640" rx="10" fill="${glass}"/><rect x="3242" y="272" width="186" height="640" rx="10" fill="${glass}"/>
<g clip-path="url(#body${paint.slice(1)})">${decals.over}</g>
${[1040, 2720].map((x) => `<circle cx="${x}" cy="1080" r="190" fill="#c9cfd8"/><circle cx="${x}" cy="1080" r="150" fill="#16181d"/><circle cx="${x}" cy="1080" r="78" fill="#8b939e"/><circle cx="${x}" cy="1080" r="26" fill="#16181d"/>`).join('')}
<rect x="120" y="1076" width="3780" height="10" fill="#0003"/>`;
};
const light = busSide('#f4f6f9', {
  under: img('03-swoosh/stippen-verloop-blauw', 120, 660, 1700, 425) + img('03-swoosh/biesjes-blauw-oranje', 120, 1000, 3780, 50),
  over: img('01-logo/logo-op-licht', 1180, 640, 1300) + img('05-teksten/website-op-licht', 2540, 770, 400) +
    ['1', '0', '2', '4'].map((d, i) => img(`04-wagennummers/zwart/cijfer-${d}`, 3600 + i * 50, 820, 56, 80)).join('') +
    img('07-pictogrammen/instappen', 3050, 330, 130, 130) + img('07-pictogrammen/rolstoel', 760, 330, 130, 130) + img('07-pictogrammen/kinderwagen', 970, 330, 130, 130) + img('07-pictogrammen/uitstappen', 3260, 330, 130, 130),
});
const dark = busSide(NAVY, {
  under: img('03-swoosh/swoosh-wit', 120, 700, 3780, 380),
  over: img('01-logo/logo-op-donker', 1180, 640, 1100) +
    img('07-pictogrammen/instappen', 3050, 330, 130, 130) + img('07-pictogrammen/rolstoel', 760, 330, 130, 130) + img('07-pictogrammen/uitstappen', 3260, 330, 130, 130) +
    img('04-wagennummers/wagenbord-1024', 3510, 800, 340),
});
const preview = `<!doctype html><html><body style="margin:0;background:#dde2ea"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 4000 2700" width="2400" height="1620"><g transform="translate(0 0)">${light}</g><g transform="translate(0 1350)">${dark}</g></svg></body></html>`;

const sheetPng = join(out, '_overzicht.png');
await view.screenshot({ path: sheetPng, fullPage: true });
await view.setViewportSize({ width: 2400, height: 1620 });
writeFileSync(tmp, preview);
await view.goto(rel(tmp));
await view.screenshot({ path: join(out, '_voorbeeld.png') });
await browser.close();
rmSync(tmp);
console.log(`${made.length} decals in ${out}`);
