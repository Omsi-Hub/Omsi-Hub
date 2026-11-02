# The companion as an employee app: handing over the design decisions

Written at the end of a brainstorm session with Luc (31 October 2026). Nothing was built or
changed: this file is the whole outcome. The next session **takes the open decisions below with
Luc, one at a time**, writes down each answer here, and only then plans the build. Luc writes in
Dutch; answer him in Dutch.

## What Luc asked for (his words)

> Wat het moet doen: een Employee Management App.
>
> Een portable versie van je omsi hub account. gegevens, diensten etc
> Een toevoeging aan de rit, een full scale navigatie, haltelijst, passagier tevredenheid
> Een mooi inlog systeem met dienstpas en code
> Een slide to unlock om je code dan in te voeren wanneer je een dienst gaat starten
>
> De app heeft eerst een welkomspagina, Wanneer verbonden met een company komen er aangewezen
> diensten te staan in de app op een pagina, Er moeten popups kunnen komen vanuit het
> managament, dit kan scripted zijn door de pc of spelers die elkaar een bericht willen sturen in
> de company mode, we moeten dus goed gaan uitdenken hoe we de company mode in de app verwerken,
> zodat het een echte employee management wordt wanneer de speler kiest om in het company menu te
> gaan:
>
> wanneer spelers een andere speelmodus kiezen, komt er een scherm met wacht op open-omsi en
> wanneer open omsi geladen is het inlog scherm:
>
> ook moet er een settings menu komen met verschillende settings voor de app, zoals het in en
> uitschakelen van het inloggen met je dienst code

## What already exists - read these first

The companion is far along; the job is turning a game helper into an employee app, not starting
over. Read [COMPANION.md](COMPANION.md) (the protocol) and [COMPANION-HANDOFF.md](COMPANION-HANDOFF.md)
(what was proven on the real game) before anything else.

| What | Where |
| --- | --- |
| The server the app pairs with (LAN, port 47811, pairing code + QR, long-polled state, commands) | `crates/hub-companion` (`state.rs`, `server.rs`, `http.rs`, `pairing.rs`) |
| Run by the launcher; reads the map, the duty, the pass | `crates/hub-api/src/companion.rs` |
| The plugin side in the game | `plugin/companion.lua` |
| The app (Expo SDK 58, iPhone, iPad, Android, and a web build the hub serves) | `C:\Omsi-hub\Omsi-hub-companion-app` (repo `Omsi-Hub/Omsi-hub-companion-app`), its `docs/PROTOCOL.md` = our `COMPANION.md`, keep both the same |
| App screens | `src/app/(tabs)/` `index` (Ride), `stops`, `duty`, `messages`, `more`; `src/app/` `pair`, `connect`, `settings`, `break`, `ibis`, `printer` |
| Signing on: personnel number + code, then sign the duty order | state `signon.stage` (`signon` → `order` → `work`), command `sign_on`, `accept_order`; app `src/components/SignOn.tsx`, `Keypad.tsx` |
| The personnel pass is the **account's** (6-digit number, 4-digit code, Supabase `private.personnel`, synced to the PC's `personnel.json`) | [ONLINE.md](ONLINE.md) "The personnel pass" |
| Messages with priority `info` / `instruction` / `urgent` and answers; sources: control desk player, AI dispatcher, a named manager of "Play together", a driver | `crates/hub-core/src/company/messages.rs` |
| What a phone may order in the company (build, workshop job, replies) and the driver's crew requests (duty swaps, sick call, days off, handover note, defect) - a queue the launcher applies, the phone never writes the company | `crates/hub-core/src/company/remote.rs` (`Order`, `take`, `companies/<id>.orders.jsonl`), `crew.rs`, `crates/hub-api/src/company/crew.rs` |
| Starting the game with exactly one duty of the company's plan | `crates/hub-core/src/company/duty_launch.rs` (`DutyRef`, `Start`, `Hitch`) |
| Pay by the hour, overtime, week guarantee (a payslip's source) | `crates/hub-core/src/company/pay.rs` |
| Trainings, licences, reputation, complaints | `training.rs`, `licences.rs`, `reputation.rs`, `complaints.rs` in `crates/hub-core/src/company/` |
| Online: accounts (email, Google, Discord), friends, presence (`hub_poll` with a digest), invites, cloud saves (Storage, 2 slots), the company board | [ONLINE.md](ONLINE.md), `crates/hub-online` |

Two facts that shaped the brainstorm:

- The **company belongs to the launcher**: a file it keeps open and saves, and only it applies the
  rules. Nothing online can change a company directly.
- Requests to Supabase are watched closely: `hub_poll` was introduced on 2026-10-26 because the
  old beat was ~70% of the project's requests and most of its egress. Anything new online must be
  cheap (small, sent on change, read with a digest).

## The target flow (agreed as the direction)

```
Welcome (first start, pair) → Connected to the hub → which mode is the launcher in?
  Company mode:  Employee home (duties, messages, roster)
                 → "Slide to start duty" → PIN → sign the duty order (clock in)
                 → Waiting for openOMSI (the hub starts the game with that duty)
                 → Ride
  Other modes:   Waiting for openOMSI (the player starts it) → sign-on screen (optional) → Ride
Ride: navigation, stop list, passenger satisfaction
```

**The one protocol addition everything hangs on:** the state says which mode the launcher is in,
also while the game is not running - e.g. `hub.mode`: `menu` | `company` | `career` | `free` |
`together`. Today the app only knows `game: true/false` and `duty.kind`. (Checked: there is no
such field in `crates/hub-companion/src/state.rs`; its only `mode` is `IBIS_mode`.)

**Mock-ups** of these screens (generated with Higgsfield, GPT Image 2.5, in the app's colours from
`src/theme.ts` and the brand mark; a picture of the direction, not a spec - names, lines and
numbers in them are invented): [companion-mockups/](companion-mockups/), `01-welkom` to
`10-instellingen`.

### The pieces as brainstormed (proposals, not decisions)

- **Welcome**: three cards of explanation on the first start, then pairing; later starts go
  straight to connecting; "PC not found" with re-pair.
- **Personnel pass** as a card: front avatar, name, number, role (driver, planner, manager),
  company logo and colour, "employed since"; back (tap to flip) licence categories, trainings, a
  QR code. Apple/Google Wallet later (needs server-side signing).
- **Employee home (company mode)**: today's duty as a big card (line, tour, bus, depot, start,
  countdown) and the week under it; roster with swap requests (accept/decline), leave, sick call;
  payslip per week/month from `pay.rs`; my figures (punctuality, km, satisfaction, career level);
  company news.
- **Starting a duty**: slide with haptics → PIN pad (`Keypad.tsx`; the server already limits ten
  `sign_on` a minute; biometrics as an alternative) → the duty order (line, tour, bus, depot, first
  departure, notes of the day) → "I take this duty" = **clocking in on the game's clock**, late
  clock-in counts towards punctuality → the hub starts the game with that duty (`duty_launch`).
- **Waiting for openOMSI**: visible steps - hub connected ✓ → openOMSI started → map loading →
  ready; meanwhile tips, the line or bus coming up.
- **Passenger satisfaction**: worked out by the hub, not the app (one score everywhere, can feed
  reputation and complaints). Inputs: delay per stop; comfort (jerk and lateral acceleration from
  `/api/pos`, 30 a second); a stop passed with `stop_requested`; doors open while moving; load
  (`passengers`). App: a live gauge and a short summary after each trip ("−4 hard braking at
  Kunsthalle").
- **Pop-ups**: `info` = Inbox badge; `instruction` = a card over the ride with accept/decline;
  `urgent` = full screen, sound, vibration, must be acknowledged. While driving (> ~5 km/h) only a
  small banner, the full message at the next stop; optional read-aloud; quick replies always one
  tap.
- **Settings**: sign-on (code on/off, slide on/off, biometrics, change code); notifications
  (behaviour while driving, sound, vibration, read-aloud); ride (screen on, north-up or heading-up,
  satisfaction gauge on/off); look (theme, language - nl/de/en/es exist, text size); connection
  (paired PCs, more than one, unpair, port); privacy (status visible to colleagues in Play
  together).

## The open decisions - the next session's work

Take them **in this order** (each one changes the next), one per message, with Luc. Give him the
options, the recommendation and why, and let him choose; then write the answer and the date under
the decision here, in his words where he gives them. Don't build anything before the first three
are settled.

### 1. Local or online? - decided: hybrid (see "Decisions taken")

The brainstorm's last step; Luc was asked "hybrid?" and asked for this handover instead.

- **A. Local only** (as now): free, private, simple - but with the PC off the app is empty: no
  roster on the train, no notice when a colleague wants to swap. "Your account in your pocket"
  cannot be done.
- **B. Everything online**: no. The ride sends 30 positions a second and the device screens ten
  stands a second; through the cloud that is slow and expensive (see `hub_poll` above).
- **C. Hybrid (recommended)** - *what the game needs stays local, what your account is goes
  online*:

  | Local (home network, as now) | Online (Supabase) |
  | --- | --- |
  | Ride, navigation, stop list | Profile and personnel pass (exists) |
  | IBIS, printer, devices | Roster: a snapshot of your duties |
  | Slide → code → start the game | Figures, payslip, satisfaction per trip |
  | Live pop-ups during the ride | Inbox: messages that reach you away from home |
  | Signing on in the game (`sign_on`) | Requests: swap, sick, leave |

  How it would work: the **PC publishes, the app reads** (a few KB on change - the week's duties,
  the figures, the payslip - read with a digest like `hub_poll`; separate from cloud saves, so a
  local-save company works too). **Requests are a queue**: the same pattern as `remote.rs`, the
  queue online instead of beside the file; the launcher takes it in at start or its beat and
  applies the same rules; the app shows "waiting for your PC" until then. **Push**: a device
  token table and an edge function calling Expo Push on a new message or swap request.
  **Bonus**: an app signed in to the PC's account can pair by itself - the PC puts its LAN address
  and pairing key on the account (row-level security, own rows only); traffic stays local.

  Consequences: an account stays optional (without one the app is local, the online tabs say
  "sign in to take your account with you"); new online data (roster, payslip, device tokens)
  must go into [PRIVACY.md](PRIVACY.md) and under the existing consent; messages between players
  of Play together run through the host's PC now - an online mailbox could deliver them to an
  offline player later, not in a first version.

### 2. One code or two? - decided: the same code (see "Decisions taken")

Largely answered by the account: the pass and its code are already the account's and the plugin's
`sign_on` uses them. Open: is the app's PIN the same code (recommended: yes, biometrics as the
quick alternative), or a separate "phone lock" code? And for a player without an account: the
PC's own pass, as today.

### 3. Does the app start the game? - decided: yes (see "Decisions taken")

Should "I take this duty" make the hub start openOMSI with that duty (`duty_launch`, recommended
for company mode - it is what makes the ritual feel real), or does the player start the game and
the app link up after? For other modes the flow already says the player starts it. Check what
`duty_launch`'s `Hitch` reasons mean for the app (a duty that cannot be driven now must say why on
the phone).

### 4. Free text between players? - decided: templates only (see "Decisions taken")

`remote.rs` deliberately allows no free text in orders. Chat between players in Play together:
templates only (safe, no moderation), or free text with a report button (moderation, abuse,
privacy). Recommended: templates first.

### 5. Passenger satisfaction: a company figure or only feedback? - decided: feedback first (see "Decisions taken")

A new score that counts for the company (reputation, complaints, the board) or only feedback for
the player. Recommended: computed in the hub either way; whether it counts is Luc's call. If it
counts, the balance needs care (harsh braking in OMSI is common with some buses' physics).

## After the decisions: the proposed order

1. Local: `hub.mode` in the state and the two paths with the waiting screen (the skeleton).
2. Personnel pass card, slide to unlock, PIN, duty order, clock-in.
3. Online, read only: pass, roster snapshot, figures in the app.
4. Online inbox and push.
5. Online requests through the queue.
6. Pairing through the account.
7. Passenger satisfaction (can move earlier; it is local-only work in `hub-companion`).

Turn this into a plan with Luc before building; each step touches both repositories, and
`COMPANION.md` / the app's `docs/PROTOCOL.md` change together.

## Rules for this work

- No AI attribution in commits or pull requests (no `Co-Authored-By` Claude, no "Generated
  with Claude Code").
- The protocol text lives twice (here and in the app); keep them identical.
- The phone never writes the company: everything goes through a queue the launcher applies.
- Online: every write through `SECURITY DEFINER` functions taking the user from the JWT, RLS on
  every table, nothing the REST API should not show (see ONLINE.md "Security").

## Decisions taken

*(Fill in as Luc decides: number, choice, date, his words.)*

1. **Local or online: C, hybrid** (1 November 2026). What the game needs stays on the home
   network (ride, navigation, devices, signing on, live pop-ups); what the account is goes online
   (pass, roster snapshot, figures and payslip, inbox, requests through a queue), published by the
   PC on change and read with a digest. Luc: "C: hybride".
2. **One code or two: A, the same code** (1 November 2026). The app's PIN after "slide to start
   duty" is the personnel pass's own 4-digit code (the account's, checked by `sign_on`), with Face
   ID / Touch ID as the quick alternative (the code kept in the device's Keychain / Keystore,
   `expo-local-authentication`, a new build); signing on with the code can be turned off in the
   settings. Without an account: the PC's own pass, as today. Luc: "we doen optie A dezelfde code".
3. **Does the app start the game: yes** (1 November 2026). In company mode "I take this duty"
   makes the hub start openOMSI with exactly that duty (`duty_launch`); a duty that cannot be
   driven now shows its `Hitch` and what to do on the phone. Other modes: the player starts it.
4. **Free text between players: no, templates only** (1 November 2026). Ready-made messages in
   every language; no moderation needed.
5. **Passenger satisfaction: feedback first, counting later** (1 November 2026). Computed in the
   hub; at first only the player's gauge and the summary after each trip; it counts for
   reputation and complaints only once the balance is right.

Further, settled the same day so the work can go on without asking:

- **The brainstorm's proposals are the direction** (welcome cards, the pass as a card that flips,
  employee home with duty, roster and payslip, the waiting screen with its steps, the settings
  menu); Wallet passes later.
- **Pop-ups as proposed**: `info` an Inbox badge, `instruction` a card over the ride with
  accept/decline, `urgent` full screen with sound and vibration; above ~5 km/h only a small
  banner, the full message at the next stop.
- **First the navigator's map, then the employee app.** The navigator shows openOMSI's own map
  one to one, in 3D (tilted behind the bus, the buildings standing; a button for flat 2D, the
  choice kept), with every bus on the map (AI and players) as a pin with its line number and the
  passengers waiting at the stops.
- **The map must be smooth and responsive** (Luc: "laat het ervoor zorgen dat de kaart bij zoomen
  en slepen super smooth en responsive is"), an acceptance requirement: gestures on the UI thread
  only, a pinch about the fingers, gliding on after a release, nothing loaded while a finger is
  down, no jump as a drag begins, lines and labels following every frame; tried on the iPad at
  its 120 Hz (the app's `docs/NAVIGATOR-MAP.md`, "Acceptance").
- **Working rules**: commit and push to the working branches freely, never merge to main or merge
  a PR without Luc; the companion app, Omsi-Hub and the openOMSI fork may all be changed (no
  upstream openOMSI pull request without Luc); development builds with EAS may be started (no
  store builds or submissions); Supabase: migrations and edge functions written in the repo and
  tried on a Supabase branch, applied to production by Luc.
