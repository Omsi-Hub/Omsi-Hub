# The phone and tablet companion (API 1)

How the add-on talks to the **Omsi-Hub Companion** app (iPhone, iPad, Android; its own
repository, `Omsi-Hub/Omsi-hub-companion-app`, keeps the same text as `docs/PROTOCOL.md`). The
add-on is the server, on the player's computer; the app is a client on the same home network.

```
 openOMSI ── plugin (Lua) ──UDP 127.0.0.1:47812──▶ Omsi-Hub add-on ◀──HTTP :47811── Companion app
                 ▲                                       │
                 └──── data/companion.json (commands) ───┘
```

* The **plugin** (`plugin/companion.lua`) reads the game twice a second (the devices' screens
  ten times) and sends it to the add-on as small JSON datagrams (`omsi.send`, at most 8 KB
  each). It reads the commands the add-on writes into its data folder (`data/companion.json`)
  four times a second and carries them out in the game (IBIS keys, the ticket printer, the
  devices' taps, signing on, taking a duty).
* The **add-on** (`crates/hub-companion`, run by `crates/hub-api/src/companion.rs`) keeps the
  latest state, adds what only it knows (the company's duty, the driver's pass, the control
  room's messages, the map's roads and the trip's route, the next turn, the bus's devices read
  from its own files by `crates/hub-devices`) and serves it to the paired devices.
* The **app** shows it and sends commands.

Everything is JSON, UTF-8, `snake_case`. Times of day are **seconds since midnight** of the game's
clock (`30240` = 08:24:00); durations are seconds; distances metres; map coordinates are the
game's (x east, y north).

## Finding and pairing

The server is off by default. The player turns it on in the add-on under
**Settings › General › Phone & tablet**; the launcher then shows the address, a six-digit
**pairing code** and a QR code. The QR code holds the web app's pairing page

```
http://192.168.1.20:47811/pair?code=482913
```

which the phone's camera opens in the browser; the app's own scanner reads it too, and also
`omsihub://pair?host=192.168.1.20&port=47811&code=482913&name=Luc-PC`. Default port **47811**.

### The web app

On the same address the server serves the Companion app's own web build (`expo export
--platform web`), so a phone or tablet without the app has all of it in its browser. Every
`GET` outside `/api/` is the web app (`crates/hub-companion/src/web.rs`): one of its files, or
for a path without an extension (`/`, `/pair`, `/duty`) its page, which routes it itself. It
needs no key; the page pairs with the code like the app (`POST /api/pair`, the device named
after its system and browser, "Android · Chrome") and keeps the key in the browser's local
storage. Its files are embedded in the add-on gzipped (`assets/companion-web`, made by
`node tools/companion-web.mjs` from the app's repository: run it when the app changes), sent
with `Content-Encoding: gzip`; the hashed ones are cached for good, the page never.

### Windows' firewall

A player never deals with it. Windows lets nothing reach a program it has no rule for; the
first time a program listens it asks - for private networks only, while a home network is often
called public - and a question clicked away leaves a rule that blocks the program for good. So
on Windows the add-on looks, while the server runs, whether the phones get in (`hub-api`'s
`companion::firewall`, PowerShell's `Get-NetFirewall*`, whatever language Windows speaks): its
own rule *Omsi-Hub Phone & tablet* letting the port in, and no rule blocking the program. When
the player turns Phone & tablet on and they would not get in, it sets that right once - the
rules blocking the program removed, its own rule for the port on every kind of network - and
Windows asks whether Omsi-Hub may (UAC). Clicked away, or found shut as the app starts, the
settings say so beside the pairing code with a button (`hub_companion_firewall`); the state's
`firewall` is `checking`, `open`, `shut`, `asking` or `declined` (none off Windows). The server
itself still lets only the home network in and only paired devices on.

| Request | Auth | Answer |
| --- | --- | --- |
| `GET /api/hello` | none | `{app: "omsi-hub", api: 1, version: "1.1.1", name: "Luc-PC", pairing: true}` |
| `POST /api/pair` `{code, device}` | none | `200 {key, name}`; `403 {error: "wrong_code"}`; `429 {error: "too_many"}` (ten tries a minute) |

`device` is the phone's own name ("Luc's iPhone"), shown in the launcher's list of paired
devices, where each can be removed. `key` is 32 hex characters; the app keeps it (secure
storage) and sends it on every other request:

```
Authorization: Bearer <key>
```

A request without a valid key gets `401 {error: "not_paired"}` (the app then pairs again).

## State

`GET /api/state?after=<v>` is a **long poll**: answered at once when the server's state is newer
than `v`, else as soon as it changes, else after 20 s with the same state. The app asks again at
once after every answer (and backs off 2 s → 10 s after errors). `after` omitted: at once.

```jsonc
{
  "v": 1842,                       // version of this state; send it back as ?after=
  "game": true,                    // the plugin was heard in the last 5 s, or within the hour while the game the add-on started still runs (paused, out of focus)
  "clock": 30240.5,                // the game's time of day
  "paused": false,                 // the game's pause menu, or the game there but quiet
  "signon": {                      // signing on (the driver's pass is the add-on's)
    "stage": "signon",             // "signon" | "order" | "work" | "none" (no sign-on needed)
    "number_hint": "48…",          // never the full number or the code
    "error": null                  // "wrong" after a wrong number/code
  },
  "duty": {                        // null: no duty at all
    "kind": "company",             // "company" | "shift" | "tour" | "free" | "game"
    "name": "Duty 14",
    "line": "M29", "tour": "3",
    "start": 30120, "end": 52200,
    "depot": "Bismarckplatz",
    "company": "Stadtbus",         // the company's name ("" on a drive of the player's own)
    "started": true,               // the game drives the duty (false: assigned, not taken yet)
    "trips": [
      { "n": 1, "line": "M29", "destination": "Hauptbahnhof",
        "departure": 30240, "arrival": 44640, "state": "done" }  // "done" | "now" | "next" | "later"
    ]
  },
  "trip": {                        // the trip being driven; null between trips / without duty
    "n": 3, "of": 13,
    "line": "M29", "destination": "Markgraf-Berthold-Platz",
    "departure": 30240, "arrival": 44040,
    "delay": 120,                  // seconds late, early negative
    "next": 4,                     // index into stops of the next stop
    "at_stop": false,              // the bus stands at the next stop
    "stops": [
      { "name": "Hauptbahnhof", "arrival": 30240, "departure": 30240,
        "delay": 120, "passed": false, "calls": true, "x": 1203.5, "y": -88.2 }
    ]
  },
  "bus": {                         // null on foot
    "name": "MAN Lion's City", "number": "2604",
    "x": 1180.2, "y": -64.0, "heading": 92.0, "speed": 32.5,
    "limit": 50,                   // the speed limit of the lane it drives on (km/h), null off the lanes
    "camera": 120,                 // a speed camera on the lane ahead (m, within 250 m), null without
    "offence": { "n": 3, "kind": "speeding", "speed": 63, "limit": 50, "fine": 3000 },  // the last offence the drive watch saw (numbered; "red_light" or "speeding", fine in cents), null before any
    "doors_open": false, "stop_requested": true, "passengers": 23,
    "light": { "aspect": "red", "change_in": 7.5, "distance": 42.0 }  // the traffic light ahead (within 150 m): "red" | "red_yellow" | "green" | "green_yellow" | "yellow" | "dark", null without
  },
  "nav": {                         // null without a route
    "maneuver": "left",            // "straight" | "slight_left" | "left" | "sharp_left" |
                                   // "slight_right" | "right" | "sharp_right" | "uturn" |
                                   // "roundabout" | "arrive"
    "distance": 200,               // metres to it
    "street": "Rathausstraße",     // the road after it, where the map names it; else ""
    "toward": "Kunsthalle",        // the next stop (always set)
    "route_v": 7                   // GET /api/route again when this changes
  },
  "pause": {                       // a break between trips; null when none
    "active": true,
    "since": 29500, "until": 29960,
    "place": "Bismarckplatz",
    "next": { "departure": 30120, "line": "307", "destination": "Hauptbahnhof" },
    "auto": true                   // started by itself (the bus stood at the terminus)
  },
  "messages": {
    "unread": 2,
    "items": [
      { "id": 77, "time": "12:58", "from": "Control centre",
        "priority": "urgent",      // "info" | "instruction" | "urgent"
        "text": "Diversion Kunsthalle: use Ringstraße until 10:00",
        "state": "delivered",      // "delivered" | "read" | "accepted" | "declined" | "sent"
        "state_label": "Delivered",
        "mine": false,
        "answers": [ { "key": "read", "label": "Acknowledge" } ],
        "action": null,            // an instruction's words ("Short-turn at Rathaus"); null: none
        "reply": "" }
    ],
    "quick": [ { "key": "late", "label": "Running late" } ]  // quick buttons to control
  },
  "ibis": {                        // the bus's IBIS; null when the bus has none
    "kind": "stock",               // "stock" (the stock MAN/Krüger IBIS keys) | "generic"
    "line": 29, "suffix": 0, "kurs": 3, "route": 1,
    "ziel": 145, "terminus": "Markgraf-Berthold-Platz",
    "stop": "Kunsthalle", "stop_index": 4,
    "mode": 0,                     // IBIS_mode as the bus's script has it
    "entry": "",                   // what is typed, where the bus shows it
    "display": ["29/03  145", "Markgraf-Berthold-Pl."], // the display's lines, where readable
    "keys": ["IBIS_0", "IBIS_1", "…", "IBIS_eingabe", "IBIS_loeschen"]
  },
  "printer": {                     // the ticket printer; null when the bus sells no tickets
    "tickets": [ { "i": 1, "name": "Single", "price": 2.9, "day": false } ],
    "request": { "name": "Single", "price": 2.9 },  // the passenger at the desk; null: nobody
    "sales": { "count": 14, "money": 40.6 },
    "currency": "€",
    "keys": ["ticketprinter_button_ticket_1", "…"],
    "change_keys": []              // the change dispenser's triggers, where the bus has one
  },
  "hub": { "mode": "company", "…": "…" },   // the launcher (below)
  "features": { "ibis": true, "printer": true, "nav": true, "messages": true, "signon": true }
}
```

Every key may be missing or `null` where the game, the bus or the add-on does not know it; the
app shows what it has and hides the rest.

### The launcher

`hub` is Omsi-Hub's own side, there with the game or without it (null from an Omsi-Hub before
it). The phone follows the player through the launcher with it: a company's duties on its home,
the menu, a game of his own.

```jsonc
"hub": {
  "mode": "company",             // "company" (his own company open), "together" (a company joined
                                 // with "Play together"), "free" (a game of his own), "menu"
  "signed_in": true,             // the Omsi-Hub account on the PC
  "user": "…",                   // its user id (null: signed out, or not fetched yet)
  "account": "Sanne",            // its profile's display name
  "company": { "name": "Stadtbus Grundorf", "short": "SBG", "colours": ["#1f68ea", "#ffffff"] },
  "role": "owner",               // "owner", or a joined player's "driver" | "planner" | "manager" | "co_owner"
  "today": "2026-11-06",         // the company's day, and the time of it (seconds)
  "now": 27720,
  "duties": [                    // his duties still to drive, the next first (two weeks at most)
    {
      "key": { "date": "2026-11-06", "line": "M29", "tour": "3", "duty": 0 },  // for start_duty
      "date": "2026-11-06", "line": "29", "colour": "#e3000f", "tour": "3", "duty": 1,
      "start": 30240, "end": 52200, "trips": 9,
      "bus": "2604 MAN Lion's City",   // null: none planned yet; "rental"
      "state": "mine"            // a joined player's only: mine, asked, driving, late
    }
  ]
}
```

The company mode is the signed-in player's (Luc, 2 November 2026): signed out on the PC, the
company is named and `duties` is empty, and `start_duty` answers `not_signed_in`. The app asks
for the same account itself (its own sign-in, Supabase Auth) before it shows the company's part.
`hub` is looked at every 3 s.

## Map

| Request | Answer |
| --- | --- |
| `GET /api/route` | the trip's route: `{v, pts: [[x, y], …], stops: [{name, x, y}]}` (`v` = `nav.route_v`) |
| `GET /api/roads?x=&y=&r=` | the roads within `r` m (≤ 2000) of a point: `{roads: [{w, pts: [[x, y], …], lanes: [[offset, way, turns]], j}], water: [], green: [], names: [{name, x, y, angle}], entries: [{name, x, y}]}` - each road a carriageway: its `lanes` how far right of its line (m, the way the points run), which way (1 with the points, -1 against) and where the lane may go at its end (1 left, 2 straight on, 4 right); `j: true` and no lanes for a junction's own piece - `names`: the streets' names as openOMSI's navigator says them (each name sign's name carried along its road every 220 m; `angle`: the way the road runs, degrees anticlockwise from east), `entries`: the map's entry points - `lights`: every traffic light of the map there, where it holds the traffic - `[x, y, heading, width]`, the middle of a lane's stop line, the way the lane runs over it (degrees clockwise from north) and its width (what each shows comes with `/api/traffic`'s `lights`, for those near the bus) |
| `GET /api/lines` | the map's lines and tours, for `take_duty`: `{lines: [{name, tours: [{number, today}]}]}` |
| `GET /api/pos?after=<n>` | where the bus is, thirty places a second: `{pos: [{n, t, x, y, heading, speed}, …]}`, those after the `n`-th (without `after`: the latest), at once when there is one, else within 2 s (`pos: []`) |
| `GET /api/ground/index` | every square of the map with something in it and the version they are in: `{v, size: 500, squares: [[i, j], …]}` - a phone fetches the whole map once and keeps it under `v` (another `v`: the game wrote the map's picture anew, or the add-on packs squares otherwise - fetch again); 404 when the add-on has none of this map |
| `GET /api/ground/<i>/<j>[?low=1][?dress=1][?gz=1]` | the game's picture of the map in the square of 500 m whose lower corner is at `i`·500, `j`·500 (the app's coordinates): binary, see below (gzipped for a client that takes it); `low=1` a square far off, without the paint, the trees and the houses' models; `dress=1` the houses' models dressed in their own textures (below); `gz=1` the gzip itself as the body (`application/gzip`, no `Content-Encoding`), for a phone that keeps it so; 404 when the add-on has none of this map |
| `GET /api/ground/skin/<id>` | a house's texture, 128 × 128 pixels: a JPEG, or a PNG where its clear parts are holes (a fence, leaves); 404 for an id the add-on did not give |
| `GET /api/traffic` | what else is on the road, as the plugin last wrote it (every second): `{at, vehicles: [{id, x, y, heading, kind}], buses: [{id, x, y, heading, line, tour, trip, terminus, delay, number, at_stop}], players: [{name, x, y, heading}], lights: [{x, y, heading, width, aspect, change_in}]}` - `lights` the traffic lights within 500 m of the bus where they hold the traffic (the middle of a lane's stop line, the way the lane runs over it, its width), what each shows as `bus.light`'s `aspect` and the seconds to its next change (-1: none coming); empty with an openOMSI without `omsi.traffic.lights` |

The app fetches roads in tiles of 1000 m and keeps them; `w` is the road's width in metres.

**The ground** is openOMSI's own picture of the map, the one its navigator draws. The game
makes it from the map's tiles once and keeps it in `~/.openomsi/cache/mapgen/<map>.bin`
(`OMML` version 19). The add-on reads that file. A map the game has not drawn yet, or a cache of
another version, has no ground (404), and the app draws the roads alone.

A square's answer is little-endian. Positions are `i16` in sixteenths of a metre from the
square's lower corner; a triangle reaching out of its square still fits.

```
"OHG1"  u16 units a metre (16)  u16 classes (13)
13 ×   u32 n, n triangles of 3 × (i16 x, i16 y)
           land: grass, forest, field, paved, sand, water;
           paving: road, foot, area, green, inner; paint: white, amber
u32 n   houses:  u16 points, points × (i16 x, i16 y) (anticlockwise),
                 u16 roof triangles, × 3 u16 (indices into the outline),
                 u16 height in decimetres, u8 flags (1: open - a roof on posts, no walls;
                 2: its model comes below, so it is not raised from its outline)
u32 n   trees:   i16 x, i16 y, i16 the crown's radius
u32 n   lines:   u8 class (0 rail, 1 tram, 2 river, 3 road edge, 4 area edge),
                 u16 width (sixteenths of a metre), u16 points, points × (i16 x, i16 y)
u32 n   model triangles: 3 × (i16 x, i16 y, i16 z), u8 shade - the houses' own models,
                 placed (z up from the ground); the shade below 128 a wall's light (0 turned
                 from it, 127 facing it), from 128 a roof's darkening as it turns away (0 to 127)
```

The models are openOMSI's own of the houses (untextured, as its 3D map draws them), at most
40 000 triangles a square, the nearest its middle first; a house whose model did not fit is
raised from its outline. After them come what stands at the stops in the square (the game's
picture of the map has none): the objects within 15 m of a stop - its shelter, its sign, the
bins and benches, the masts and fences - as the map's own models read from their `.sco` (not a
plant, a road's piece, a mark on the ground or a building more than 20 m across or 7 m high),
not in a square far off. An add-on from before them ends the square after the lines.

With `dress=1` the houses come dressed as the game dresses them: the add-on finds each house's
own object on the map again (where it stands and which way it faces, as the game's picture
placed it) and reads its model with its textures. A dressed house's triangles leave the model
triangles above and come after everything in a section of their own - the same faces, at most
40 000 a square as before:

```
"SKN1"  u16 looks: u32 texture (its id for /api/ground/skin, 0: none - the colour alone),
                 4 × u8 colour (r, g, b, a: the material's where it has no texture),
                 u8 flags (1: the texture's clear parts are holes)
u32 n   triangles: 3 × (i16 x, i16 y, i16 z), 3 × (i16 u, i16 v) - where on the texture,
                 2048 a repeat (it repeats: each triangle's moved by whole repeats to start
                 near 0) - u16 look, u8 shade (as a model triangle's)
```

The textures come apart, once each however many houses wear them; the app keeps them on its GPU
while a square it shows wears them. A house whose object is not found (or an add-on from before
it, which leaves out the section) comes as a model triangle, untextured.

Each triangle, house and tree is in the square its middle is in. A line comes with every square
it passes, by the stretch of it there.

The bus on the map moves every frame of the screen: the app asks `/api/pos` again as soon as it
has its answer (at most about twenty times a second, a few places to each) and plays the places
back a moment behind, drawing the bus between two of them (`t`: the seconds of game they came at;
`n` counts from 1 again when the game starts again). The places come from the plugin's
`data/pos.json`, read only while a device asks; they are where the state's `bus` is, thirty
times a second instead of a few. An add-on without them answers 404, and the app goes by the
state's `bus` then.

## The bus's devices, drawn as the game draws them

The IBIS and the ticket printer or ALMEX of the bus the player drives - no other screen of
the bus: the passengers' display, the dashboard's or a thermometer would only confuse in the
app, and a screen with nothing to press is left out too - are read from the bus's own files by
the add-on (its
`model.cfg` and the `.o3d` parts and textures beside it, the way Luc's career add-on for OMSI 2
did, github.com/Luc-nbr/Omsi-career-addon: `core/busmodule.ts`, `core/schermvorm.ts`; the
add-on's `crates/hub-devices` is a port of it) and sent to the app as a **form**: the pictures of
the screen's pages laid flat as triangles, the text fields with the bus's own fonts, and the
touch fields with the trigger each one fires. The app draws the form with the screen's
**stand** of this moment (the strings and numbers it shows) and a tap fires the touch field's
trigger in the game. Nothing is drawn by hand per bus.

`state.devices` lists those devices of the bus now (empty on foot):

```jsonc
"devices": [
  { "id": "17_almex",                 // stable per bus and device
    "name": "ALMEX",                  // as the bus names it (the parts' names), else the kind
    "role": "printer",                // "ibis" | "printer" ("other": an older add-on; not shown)
    "kind": "touch",                  // "touch" (its screen is the keys) | "screen_keys" |
                                      // "keys" (keys only); a display only is not sent
    "form": "a1b2c3d4e5f60718293a",   // the form's id (20 hex), null: no screen could be drawn
    "keys": [ { "trigger": "almex_riegel", "label": "RIEGEL", "digit": null, "row": 2,
                "loose": true } ]
                                      // keys of the device off its screen (keypads, the change
                                      // dispenser), as rows under the screen: "row" from the top,
                                      // as the keys lie on the device; "loose" (only when true):
                                      // of a touch device, a key that sits apart from it (the
                                      // ALMEX's latch, its change) - shown after, folded away
  }
]
```

| Request | Answer |
| --- | --- |
| `GET /api/device/<form>` | the form (below); it does not change while the bus is the same, the app keeps it by id |
| `GET /api/texture/<id>` | a picture of the form as `image/png` (ids of 20 hex characters, never a path) |
| `GET /api/screen/<form>?after=<v>` | the stand (below), long polled: at once when newer than `v`, else within 2 s |

The form (the career add-on's `shared/scherm.ts` `Schermvorm`, in English):

```jsonc
{
  "version": 1, "id": "a1b2c3d4e5f60718293a", "device": "17_almex",
  "aspect": 1.381,                    // width / height in the real world
  "blank": "#000000",                 // the colour behind everything
  "parts": [                          // in drawing order
    { "kind": "image",                // "image" | "color" | "text" | "script" (not drawable: blank)
      "mesh": 12,                     // the mesh's place in the model, or -1
      "visible": [ { "num": 0, "value": 3 } ],   // all true = shown; num: index into "nums";
                                      // shown when Delphi-round(value of nums[num]) == value; a
                                      // variable the bus lacks counts as true, NaN as false
      "tris": [ /* s t u v, three points a triangle, twelve numbers each */ ],
                                      // s, t: 0..1 across and down the screen as the driver sees it;
                                      // u, v: 0..1 in the picture (v down), wrapping per "address"
      "depth": 0.4,                   // mm towards the driver; drawn from small to large
      "alpha": 0,                     // [matl_alpha]: 0 opaque, 1 alpha test, 2 blended
      "address": "wrap",              // "wrap" | "clamp" | "border"
      "texture": "0f…",               // image: the picture's id
      "freetex": 3,                   // image with [matl_freetex]: index into "strs" whose value is
                                      // a file; the stand's "f" gives its picture id
      "choice": { "num": 2, "items": ["0e…", null] },   // [matl_change]: 0 the texture, k item k
      "color": [0.1, 0.2, 0.3, 1],    // color, and for alpha 2 the image's opacity: 0..1
      "text": {                       // text: a [texttexture] / [texttexture_enh]
        "str": 4,                     // index into "strs"
        "w": 256, "h": 32,            // the texture's pixels the game writes the text into
        "font": "HH20_HHAschedule",   // key into "fonts"
        "fc": 0,                      // field 5: bit 0 colour from the bitmap, (fc & 3) == 2 opaque only
        "color": [255, 160, 0],       // the letters' colour, 0..255
        "align": 0,                   // _enh field 9: 0 centre, 1 left, 2 right, 3/4/5 on a gap
        "grid": 1                     // _enh field 10: x rounded down to a multiple of this
      } }
  ],
  "taps": [                           // the front one last
    { "trigger": "almex_clickN7", "label": "7", "mesh": 40, "visible": [],
      "x": 0.1, "y": 0.8, "w": 0.08, "h": 0.1,   // a rectangle in shares of the screen
      "depth": 0.6 }
  ],
  "fonts": { "HH20_HHAschedule": { "name": "…", "height": 9, "sep": 1,
    "glyphs": [ { "ch": "A", "width": 6, "alpha": "<base64: max(0,width) x height bytes>",
                  "rgb": "<base64 r g b per pixel, fonts with colour>" } ] } },
                                      // glyph 0 is the fallback; file order, doubles kept
  "strs": [ "almex_display_hst" ],    // the string variables the stand sends ("t", "f")
  "nums": [ "almex_menu" ],           // the number variables the conditions use ("g")
  "incomplete": [ "17_almex_s_bg: \\S:2 script texture" ]  // what could not be drawn
}
```

The stand, ten times a second while a device is on screen:

```jsonc
{ "v": 811, "form": "a1b2c3d4e5f60718293a",
  "t": ["    LEE", "04:13:07"],          // per "strs", as the bus has it (not trimmed)
  "g": [3, 0, null],                    // per "nums"; null: the bus has it but it is no number
  "o": [5],                             // indexes into "nums" the bus does not have (always true;
                                        // missing when none)
  "f": [null, "0c…"] }                  // per "strs": the picture id of a freetex value, or null
                                        // (missing when the form has no freetex part)
```

Taps go through `POST /api/do`: `{do: "press", trigger}` presses and lets go 0.15 s of game
time later (`omsi.press`, then `omsi.release`: a script that reads its key's variable in its
frame, as the HH20's ALMEX does, never sees `omsi.trigger`'s press of one frame);
`{do: "press", trigger, down: true}` holds it and `{..., down: false}` lets it go
(`omsi.press` / `omsi.release`, for keys a script times). Only the triggers of the current
bus's devices' taps and keys are pressed; anything else is `unknown_key`.

## Commands

`POST /api/do` with one command; answered when the game has done it (at most 4 s):
`200 {ok: true}` or `200 {ok: false, error: "…"}`; `400` for a command the server does not know.

| `do` | Fields | What happens |
| --- | --- | --- |
| `sign_on` | `number`, `code` | Signs the driver on (the pass the add-on knows); ten tries a minute |
| `accept_order` | | Signs the duty order: the duty starts, the IBIS is typed |
| `drive_free` | | Drive without a duty (the plugin gives the duty up, the bus stays) |
| `take_duty` | `line`, `tour`, `trip?` | Takes a duty of the map's timetable (`omsi.duty.start`) |
| `type_ibis` | | Types the duty into the IBIS (`omsi.duty.type_ibis`) |
| `ibis_key` | `key` | Presses one of `ibis.keys` (held 0.15 s, as a tap) |
| `ibis_line` | `line` | Types a line into the IBIS (`omsi.bus.set_line`) |
| `printer_key` | `key` | Presses one of `printer.keys` or `printer.change_keys` |
| `ticket` | `i` | Sells ticket `i` of `printer.tickets` (its key, else the ticket key `ticket_give`) |
| `msg_read` | `id` | Marks a message read |
| `msg_answer` | `id`, `key` | Answers a message with one of its `answers` (`read`, `accept`, `decline` or a canned reply's key) |
| `msg_quick` | `key` | A quick button to control |
| `pause_end` | | Ends the break early |
| `press` | `trigger`, `down?` | Presses a tap or key of one of `devices` (above) |
| `start_duty` | `date`, `line`, `tour`, `duty` | Starts a company duty (a `hub.duties[].key`) as the launcher's "Start duty": the launcher's, there with the game or without it (below) |

Only the keys the state lists can be pressed: the plugin refuses any other trigger. The game
names a bus's triggers in lower case (`omsi.bus.triggers`, openOMSI 0.3.37) and presses them
without regard to case; the plugin compares them without case too.

The errors a command can answer: `game_not_running` (the plugin was not heard for 5 s),
`no_plugin`, `timeout` (the game did not ack it within 4 s), `unknown_key`, `too_many` (ten
`sign_on` a minute), `wrong` (not the driver's number and code), `not_now` (no duty order to
sign), `no_message`, `no_printer`, `not_supported` (this openOMSI lacks the function), or the
game's own reason a duty could not be taken.

`start_duty` is the launcher's, not the game's: it is checked at once and answered then. Its
errors: `not_signed_in` (the PC's account), `no_company` (no company open in the launcher),
`game_running`, `reading` (the planning is still being read: try again), `no_launcher`,
`not_supported` (a joined company's duty, not yet), `not_found`, or what keeps the duty from
starting in the launcher's words ("Nobody is planned on this duty. Plan yourself on it in
RosterFlow …"). Started, the launcher opens its duty launcher on the duty and starts it with
the player's game options, as its own button does; the game coming up shows in `game`.

## The plugin's datagrams (add-on internal)

To `127.0.0.1:47812` (`companion_udp_port`), each one JSON object with `t`:

| `t` | Every | Holds |
| --- | --- | --- |
| `live` | 0.5 s | `clock`, `paused`, `bus`, `bus_file` (the bus's `.bus`, relative to the game folder), `duty` (line, tour, trip n, delay, next stop index, at_stop, trip_done), the plugin's `signon` stage |
| `trip` | on change | the trip's stops (paged: `page`, `pages`, ≤ 25 stops a page) |
| `trips` | on change | the duty's trips |
| `dev` | 0.5 s | the IBIS and printer variables and strings, `ticket_request`, `sales` |
| `keys` | 5 s | the bus's triggers that belong to the IBIS, the printer and the change dispenser, and the map's tickets |
| `lines` | 10 s | the map's lines and tours (paged) |
| `scr` | 0.1 s | what the devices' screens show (paged under 8 KB, a new `v` when it changes): `items` of `["n", name, value]` (no value: not a number), `["o", name]` (the bus lacks it) and `["s", name, text]` |
| `ack` | per command | `seq`, `ok`, `error` |

And the add-on writes `data/companion.json` in the plugin's data folder:
`{sid, port, seq: 12, cmds: [{seq: 11, do: "key", key: "IBIS_7"}, …], probe: [x, y]}` (the
last 20 commands of the server's session `sid`, the port for the datagrams, and the place whose
street name the plugin reports in `live.probe`), and for the bus's devices `want: {nums, strs}`
(the variables `scr` reads) and `press: [trigger, …]` (the taps and keys of the devices: a
`{do: "press", key, down}` command presses only one of these that the bus really has). The
plugin carries out each `seq` of the session it has not done yet and acks it. Commands already
in the file when the plugin starts count as done. The plugin talks to the companion only while
the file is there.
