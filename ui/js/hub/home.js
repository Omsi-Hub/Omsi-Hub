// The start (the drive's "Mode" step), as our game-drawn launcher's (`flow::step_mode`): the
// logo over the greeting, the ways to drive as tiles with a photo each - a work shift, a tour,
// a free drive, and the bus company beside them, each badge in the logo's line - and under
// them the driver's service record and the ways to everything else. The glass card is as tall as what is on it; what does not
// fit scrolls inside it (`hub-card-in`), under the card's rim, which stays where the card is.

import { html, useEffect, useRef, useState } from '../../vendor/preact-htm.js';
import { useStore } from '../store.js';
import { choose } from './drive.js';
import { Icon, tr, hoursText } from './bits.js';
import { LOGO_SVG } from './logo.js';
import { go } from './hub.js';
import { openEditor, openTimetable, openDepotEditor } from '../editors/index.js';
import { goStep } from './flow.js';
import { enterCompany } from '../company/company.js';
import { togetherOn, blockedLabel } from '../company/gate.js';

/** The ways to drive (kinds: 0 a tour, 1 a shift, 2 free, 4 the bus company), as the tiles
 * say them. */
const MODES = [
  [1, 'Work shift', 'schedule', 'mode-shift', "Choose how long you want to drive and when: Omsi-Hub puts a shift together from the timetable's real trips, on with another line where lines meet."],
  [0, 'Tour', 'route', 'mode-tour', "One bus's trips on one line, as OMSI's timetable dialog gives them: pick the line, the tour and the trip to start with."],
  [2, 'Free drive', 'map', 'mode-free', "Only a map and a bus. The traffic and the timetable's buses drive around you; nothing is booked."],
  [4, 'Bus company', 'garage', 'mode-company', 'Your own transport company: buy, lease or rent buses, hire drivers and run lines day by day.'],
];

/** The greeting by the hour of the day. */
function greeting(name) {
  const h = new Date().getHours();
  const key = h >= 5 && h <= 11 ? 'Good morning %{name}' : h >= 12 && h <= 17 ? 'Good afternoon %{name}' : 'Good evening %{name}';
  return tr(key, { name });
}

/** The ring round a tile's badge (its radius), as the logo's round its bus. */
const LOOP_R = 27;

/** A tile's badge in the logo's line (`mark.svg`): the ring round it, open below on the right,
 * its foot running on as the line to the tile's far side, drawn to the tile's width. Chosen or
 * under the mouse it lights from the ring's open end round to the line's end, a ball of light
 * running along it, and the ball keeps running it (hub.css, `.hub-mode-go`). */
function ModeLine({ icon }) {
  const ref = useRef(null);
  const [w, setW] = useState(0);
  useEffect(() => {
    const el = ref.current;
    if (!el) return undefined;
    const ro = new ResizeObserver(() => setW(el.clientWidth));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  const c = LOOP_R + 4;
  const a = (35 * Math.PI) / 180;
  const foot = c + LOOP_R;
  const end = Math.max(c + 1, w - 8);
  const d = `M${(c + LOOP_R * Math.cos(a)).toFixed(2)} ${(c + LOOP_R * Math.sin(a)).toFixed(2)} A${LOOP_R} ${LOOP_R} 0 1 0 ${c} ${foot} L${end} ${foot}`;
  return html`<span class="hub-mode-go" ref=${ref} style=${`--hub-loop: path("${d}")`}>
    ${w > 0 && html`<svg viewBox=${`0 0 ${w} ${2 * c}`} aria-hidden="true">
      <path class="track" d=${d} />
      <path class="lit" d=${d} pathLength="100" />
      <circle class="end" cx=${end} cy=${foot} r="4.5" />
    </svg><i class="ball"></i>`}
    <span class="hub-mode-badge"><${Icon} name=${icon} size=${20} /></span>
  </span>`;
}

/** The way to drive chosen now (0 a tour, 1 a shift, 2 free). */
const current = (c) => (c.free ? 2 : c.composed ? 1 : 0);

function pick(kind) {
  if (kind === 4) {
    enterCompany();
    return;
  }
  choose({ free: kind === 2, composed: kind === 1 });
  goStep('map');
}

export function Home() {
  const s = useStore();
  const c = s.choice;
  const p = s.profile;
  const name = p?.name || s.config?.profile || '';
  const on = current(c);
  const level = p ? Math.max(0, Math.min(1, (p.xp - Math.max(0, p.level - 1) ** 2 * 250) / Math.max(1, p.next_level_xp - Math.max(0, p.level - 1) ** 2 * 250))) : 0;
  const links = [
    [tr('Driver: %{name}', { name }), 'person', () => goStep('profile')],
    [tr('Settings'), 'settings', () => go('settings')],
    // (the mod installer: Settings on its tab "Mods", `mods.js`)
    [tr('Install mods'), 'extension', () => go('settings', null, 'mods')],
    [tr('Timetable'), 'schedule', openTimetable],
    [tr('Multiplayer'), 'groups', () => go('multiplayer')],
    [tr('Depot editor'), 'departure_board', () => openDepotEditor()],
    // (blocked for now: `company/gate.js`)
    togetherOn() ? [tr('Join a company'), 'garage', () => enterCompany({ chooser: 'join' })] : [`${tr('Join a company')} · ${blockedLabel()}`, 'lock', () => {}],
  ];
  return html`<div class="hub-start">
    <section class="hub-card hub-glass glow"><div class="hub-card-in">
      <div dangerouslySetInnerHTML=${{ __html: LOGO_SVG }} style="display:contents"></div>
      <h1 class="hub-greeting">${greeting(name)}</h1>
      <p class="hub-question">${tr('How do you want to drive today?')}</p>
      <div class="hub-modes">
        ${MODES.map(([kind, title, icon, picture, text]) => html`<button type="button" class=${'hub-mode' + (kind === on ? ' on' : '')} data-mode=${kind} onClick=${() => pick(kind)}>
          <span class="photo" style=${`background-image:url("img/hub/${picture}.jpg")`}></span>
          <span class="hub-mode-what">
            <${ModeLine} icon=${icon} />
            <b>${tr(title)}</b>
            <p>${tr(text)}</p>
          </span>
        </button>`)}
      </div>
      <div class="hub-under">
        ${p && html`<button type="button" class="hub-record" onClick=${() => go('profile')}>
          <h3>${tr('Your service record')}</h3>
          <div class="hub-figures">
            <div><b>${p?.record?.runs ?? 0}</b><span>${tr('duties')}</span></div>
            <div><b>${hoursText(p?.hours || 0)}</b><span>${tr('driven')}</span></div>
            <div><b>${Math.round(p?.km || 0)}</b><span>${tr('km')}</span></div>
            <div><b>${p?.level ?? 1}</b><span>${tr('level')}</span></div>
          </div>
          ${p && html`<div class="hub-level">${tr('Level %{level} · %{xp} of %{next} points', { level: p.level, xp: p.xp, next: p.next_level_xp })}</div>
            <div class="hub-progress"><div style=${`width:${level * 100}%`}></div></div>`}
          <span class="hub-more">${tr('Open the service record')}<${Icon} name="chevron_right" size=${16} /></span>
        </button>`}
        <div class="hub-links">
          ${links.map(([label, icon, onClick, away]) => html`<button type="button" class="hub-link" onClick=${onClick}><${Icon} name=${icon} size=${16} /><span>${label}</span>${away && html`<${Icon} class="away" name="open_in_new" size=${14} />`}</button>`)}
          <button type="button" class="hub-link long" onClick=${openEditor}><${Icon} name="construction" size=${16} /><span>${tr("Editor: lines, liveries, the timetable and the map's objects")}</span></button>
        </div>
      </div>
    </div></section>
  </div>`;
}
