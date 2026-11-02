# The online company: a bus company that lives on a server

Status: design, nothing of it built (2026-10-31; Luc's decisions of 2026-11-01 worked in: the
owner sets the clock, one company per account, the addon list, the costs for 100 companies).
Owner: Luc. What is in this file and is not yet decided is marked **open**; what needs a change
to openOMSI is marked **upstream**.

## The goal

A bus company that is no longer a file on the boss's computer but lives on a server, always on.
Everyone who is a member of an online company can go into it whenever they like, without the
boss being online: take a duty, drive it, and see the other members who drive at the same time
in the same world. Lines the members make are the company's, kept on the server, and every member
can drive them.

The local company (single player, `co::store`, cloud saves) stays as it is. An online company is
a second kind of company, not a replacement.

## What there is today

Most of the parts exist; what changes is where they run.

- **Play together** (`crates/hub-api/src/together.rs`, `crates/hub-core/src/company/together/`):
  the boss's launcher is the company server (`server.rs`, plain HTTP, a Cloudflare quick tunnel
  in front of it). Friends join with a `HUB-…` code (`code.rs`) and post a `wire::Request`; every
  answer carries a `Snapshot` of the company's day. Roles and permissions (`roles.rs`) are checked
  on the boss's side for every message (`host.rs`). Taking, asking for, being given and releasing
  duties, trip reports, the AI taking over a duty its player left and taking it back from the
  next trip, the chat, the control room's messages, the day's close booking the players' trips
  (`book.rs`): all there.
- **Drive together** (`world.rs`): the first player who starts a duty together opens the world
  with their own game (`--lan-host`); the others join it (`--lan-join`) on its clock, from
  `EARLY` minutes before their duty. The world closes with the opener's game.
- **openOMSI's multiplayer** (`omsi-net`, `omsi-lan` in Luc's fork, the rev in `Cargo.toml`):
  - A star over UDP: each player sends their own bus up to 20 times a second (`wire`), the host
    relays it. The host alone simulates the shared world - AI traffic, people, traffic lights,
    the clock and the weather (`world`, `lan_world.rs`) - and hands waiting passengers to one bus
    only (`CLAIM`).
  - A **dedicated server** (`omsi --server server.cfg`, `omsi-app/src/server.rs`): no window, no
    sound, no graphics card. `server.cfg` has `real_time`, `time_speed`, `timetable`, `traffic`,
    `passengers`, `max_players` (up to 64), `vehicles` (the buses allowed), `tunnel`, an admin
    password and the voice chat.
  - The host's **mods go to the players who join** (`omsi-lan/src/mods.rs`): everything of the
    session that is not stock content, compared by SHA-256, sealed in a store, mounted for the
    session only.
  - A bus a player cannot load is drawn as a **stand-in** (`omsi-lan/src/game.rs`,
    `remote_type`): the viewer's own bus, or on a server the first bus its list allows.
  - **The company in the game** (`omsi-app/src/company_live.rs`, `--company`): the launcher
    writes the day (`LivePlan`: each tour with its bus, paint, fleet number and plate, or
    dropped); the timetable runs the company's tours with those buses and reports each trip once.
- **Online accounts** (`docs/ONLINE.md`): auth, `profiles`, friends, invites with a session code,
  the company leaderboard. Writes only through `SECURITY DEFINER` functions.
- **Own lines** (`crates/hub-api/src/lineeditor.rs`): the registry `~/.openomsi/lines/<map>.json`
  is the source of truth, and `core::lines::export_to_map` writes the line into the map's
  `TTData`.

## The design: two servers

```
  Launchers (members)
     │  HTTPS / WebSocket: duties, planning, lines, chat
     ▼
 ┌────────────────────────────┐   start / stop, the day's LivePlan,
 │ Company server (always on) │ ─ which tour is a player's and which the AI's
 │ hub-core's rules + Postgres│ ◄ the trips as the world saw them
 └────────────────────────────┘
     │ one world per company and map, only while someone drives
     ▼
 ┌────────────────────────────┐
 │ World server               │ ◄ UDP 20 Hz: each player's own bus
 │ omsi --server (headless)   │ ► AI traffic, people, clock, the other players
 └────────────────────────────┘
```

The two have different jobs and different costs: the company server is cheap and holds what must
never be lost; the world server is expensive and holds nothing that cannot be started again.

### The company server

- **What it runs**: `host.rs`'s `HostState::answer` taken off the boss's computer, as a process of
  its own built from `hub-core` (the same rules the launcher uses: the plan, the day's close, the
  ledger, the roles). The company's logic is **not** written again as SQL functions: it is Rust
  in `hub-core`, and two copies of it would drift apart.
- **Where the company is kept**: Postgres (the Neon project of `docs/ONLINE.md`, or the database
  the app moves to). One row per company with its state as a versioned document, the same shape
  `co::store` saves today, plus tables for what is asked for often (members, duties of the day,
  trip reports, the ledger's entries). A change is saved before it is answered.
- **Many companies, one process**: a company is small. It is loaded while a member is online or
  its clock has work to do (the day's close), and let go after that.
- **The messages**: `wire::Request` / `Snapshot` as they are, with two changes:
  - who a player is comes from the account's token (`auth.user_id()`), not from the code's
    secret; the `HUB-…` code goes, joining is membership;
  - the answers come by push (a WebSocket per launcher) instead of a poll, once there are more
    than a handful of players.
- **Concurrent edits**: every change that edits something (the planning, a line, a roster) says
  the version it was made on; one made on an older version is refused with "load it again". No
  merging.
- **The company's day goes on without anyone**: the clock, the AI's duties, the day's close and
  the catch-up with the real clock (today on a thread of the launcher, "a simulation past midnight
  or a catch-up") run on the server, on a schedule.

### The world server

- **What it runs**: openOMSI's dedicated server, `omsi --server`, with a `server.cfg` the company
  server writes:
  - `real_time` / `time_speed` as the company's clock (see "The clock");
  - `vehicles`: the company's fleet;
  - `timetable = 1` with the company's package (below) as a content root, and the company's
    `LivePlan` (`--company`): the tours nobody drives run as AI with the company's own buses, so
    the members see the rest of their company drive. **Upstream**: check that `--server` takes
    `--company`, and add a way to change the `LivePlan` while the server runs (a player takes a
    tour at 17:05: the AI's bus on it must go, or hand over where it is).
  - `max_players`, `tunnel = 1`, the admin password (the company's roles instead, later).
- **When it runs**: started by the company server when the first member starts a duty in the
  world; stopped a few minutes after the last one left. A company with nobody driving costs no
  world.
- **Where it runs**: **open** - our machines (a cost per world that runs), the company's own
  machine (as a Minecraft server: `server.cfg` exists, the company server only needs its address),
  or both.
- **Without a world server**: today's way stays as the fallback, a member's game hosts the world
  (`world.rs`). It is free, and it closes with that member's game.

## Who decides what

| What | Decided by | How it travels |
| --- | --- | --- |
| Money, fleet, staff, planning, lines, roles | the company server | `wire::Request` in, checked against the player's permissions, `Snapshot` out |
| A player's own bus (pose, doors, lamps, sounds) | the player's game | `omsi-net` `STATE`, relayed by the world server |
| AI traffic, people, lights, clock, weather | the world server | `omsi-net` `WORLD`, `CLOCK` |
| The company's tours nobody drives | the world server, from the company's `LivePlan` | drawn as AI buses for everyone |
| Which passenger boards which bus | the world server | `CLAIM` |
| Whether a trip was driven, and how | the company server, with the world server as witness | the player's report checked against what the world server saw |

## The clock

Decided (Luc, 2026-11-01): **the owner chooses** - the real clock, or a faster one.

- **Real time**: the company's clock is the real clock in the company's time zone (`server.cfg`
  `real_time = 1`). Whoever comes in at 17:00 takes a duty around 17:00 and meets the others who
  drive then. A member who only plays in the evening never drives an early duty together.
- **Faster**: the owner sets how fast (2× … 30×, `time_speed`'s range). The company's days go by
  quicker: more days, more money, more wear an evening.
- **Driving is always at 1×.** A bus drives a 20-minute trip in 20 real minutes; on a clock at 2×
  that trip would take 40 company minutes and every player would always be late. So while a
  member drives a duty of the company - in the world or solo - the company's clock runs at real
  speed, and it goes back to the owner's speed when the last one stops. A faster company is fast
  between the drives, not during them. The world server's clock follows (its admin can already
  change the clock's speed; **upstream**: a way for the company server to set it, not the chat's
  admin password).
- **Changing it**: the owner changes it in the company's settings; it takes effect at once
  outside a drive, else when the last drive ends. From real time to faster the clock goes on from
  where it is; from faster to real time it jumps to the real clock at the next day's start (the
  day being driven is not cut short).
- **Solo stays**: a member may drive an open duty of the day in their own game, without the
  others (`wire::DriveStart` without `world`). A duty is booked once: whoever's trips come first
  (the AI's in the world, or the solo player's) count, the rest is refused (`book.rs` already
  takes a duty back from the AI only from a trip it has not begun).

## The addon list

Decided (Luc, 2026-11-01): a player sees the list of the addons a company uses, compared with
their own mods, so that they know beforehand whether everything fits. This holds for an online
company and for today's Play together, where a member's launcher or game hosts.

**What is on the list** - one row per addon, not per file (an addon is a top-level folder of a
content folder: `maps/<map>`, `Vehicles/<bus>`, `Sceneryobjects/<…>`, `Splines/<…>`,
`Humans/<…>`, `Fonts`, as `mods.rs`'s `FOLDERS`):

- the map, and what it uses: scenery objects, splines, AI vehicles and people (the map's tiles and
  `ailists`, read with `hub-omsi`'s readers);
- the fleet's buses;
- the company's package (lines, depots, liveries: always sent, never the player's to install);
- per addon: its name, kind, size, a digest of its files (SHA-256 over the sorted list of each
  file's name, size and hash) and whether it may be shared (the payware mark under "A bus someone
  does not have").

Stock OMSI 2 content is left out, as `mods.rs` leaves it out.

**Who makes it**: the reference installation is the owner's, or the host's.

- An online company: the owner's launcher makes the list when the company goes online and again
  when the fleet or the map changes, and sends it to the company server. The world server checks
  its own installation against it before it opens (a world server missing an addon does not
  open, and says which).
- Play together today: the host's launcher makes it and hands it out with the session (`wire`'s
  `hello` answer, then in the `Snapshot` when it changes; a new field, so `code.rs`'s version goes
  up).
- A world a member's game opens (`world.rs`): the opener's launcher makes it.

**What the player sees** - on the company's page (a tab "Addons") and before joining:

| Mark | Means |
| --- | --- |
| ✓ | installed, the same digest |
| ≠ | installed, another version (how many files differ) |
| ↓ | not installed, sent in the world for the session (may be shared) |
| ✗ | not installed and not sent (payware, or not shareable): install it, or no duty needs it |

with a line at the top: "Everything fits" / "3 addons are sent when you join" / "2 addons are
missing: duties on these buses are not offered to you". A player can join with ↓ and ✗ rows; the
bus step does not give them a duty on a bus they lack, and solo driving needs everything that duty
uses installed (the session's sealed files are only there in the world).

**Comparing is quick**: hashing a whole map takes minutes. The player's side keeps the hashes of
its files by name, size and time changed (in the data folder), so only what changed since is
read again; the first comparison of a big map says it is reading and how far it is.

Most of this is Omsi-Hub's own code (the reading of what a map uses, the list, the page); the
installed mods come from `hub_mods` (`crates/hub-api/src/mods.rs`) and the content folders from
`hub_core::paths::content_dir`.

## A bus someone does not have

**The files.** A member who has not installed a bus of the fleet:

- The world server sends what it is allowed to (`mods.rs`, for the whole fleet instead of the
  host's one bus).
- **Payware is never sent**: sending a bought bus to someone who did not buy it is distributing
  it. Each bus of the fleet gets a "may be shared" mark (freeware yes, payware no; **open**: who
  sets it, and whether we keep a list).
- A member without a payware bus is not given a duty on it (the bus step picks another of the
  fleet), and sees it as a stand-in.
- **Upstream**: a stand-in of the same kind and length - a solo bus, an articulated one, a
  double-decker, a minibus - from a small set of free models, instead of the viewer's own bus
  (`remote_type`): a solo bus standing in for an articulated one looks wrong in a convoy.

**The vehicle.** The buses are the company's, not the players':

- A member borrows a bus for a duty: a lock on the company server, one driver per bus.
- The lock goes when the player's launcher is not heard from (`host.rs`'s offline and left
  times); the AI takes the bus on where it stands, or it goes back to the depot.
- Wear, fuel and damage are the company's costs, with the driver's name in the log.

## Lines, depots and liveries: the company's package

Today a line is written into the map's own `TTData`. For an online company that cannot work - the
members' maps would each be changed differently, and Omsi-Hub decided not to write to openOMSI's
or OMSI 2's files (`docs/PLAN.md`, "The goal").

1. **The source is the company server**: each line with its versions, who made it and when. Who
   may make or change a line is a permission of the role; a member's line can wait for the boss's
   approval and pay ("Confirm and pay" exists in the line editor's company mode).
2. **The package**: the company server builds, for each of the company's maps, one archive with
   what the company adds to the map:
   - the `TTData` of its lines (the map's own lines it runs, and its own),
   - the depots it built (`lak`, `company_more/depot.rs`),
   - its liveries (`hub-livery`).
   The package has a version (its hash).
3. **How the game reads it**: never unpacked into the map folder.
   - Solo: the launcher passes it with `--content-zip` (a content root read in place).
     **Upstream**: check that a content root wins over the map's own `TTData` files of the same
     name, and add it where it does not.
   - In the world: the world server has the package as a content root, so `mods.rs` sends it to
     the players who join, as everything non-stock of the session.
4. **A changed line runs from the company's next day**, so that the duties being driven do not
   break.
5. **The company keeps one version of each map**: a line names stops and splines of that map. A
   member whose map is another version is told so, and gets the world server's in the world.

## Trust

A player's game is the authority of its own bus, so a changed game can report anything about it.
What we do about that:

- The world server sees every player's pose. A trip report is checked against it: the stops
  served, the times, the speed (`omsi-net` clamps the values; the plausibility is ours).
- Solo trips have no witness: they count, but they are marked as solo, and the company may
  decide that only trips driven in the world count for the leaderboard.
- Everything that is not the bus - money, the plan, the roles - is decided on the server only,
  whatever a launcher says (as `host.rs` does today).

## Members and roles

New tables beside `profiles` (`docs/ONLINE.md`):

- `companies`: id, name, owner, its time zone, the maps and their versions, the state.
- `company_members`: company, user, role, the permissions ticked or unticked on top of it
  (`roles.rs`), joined, last seen. **One online company per account** (Luc, 2026-11-01): the
  user is unique in this table, as owner or as member. Joining another company means leaving
  this one first; an owner leaves only by handing the company to another member or closing it.
  The local companies of single player are not counted.
- `company_invites`: as `invites`, for a company instead of a session.
- `company_bans`.

Row-level security as in the rest: a member sees their own companies; writes go through
`SECURITY DEFINER` functions or through the company server, never straight from a launcher.

## Moving a company online

- A local company is put online once ("Make this company online"): its file goes to the server,
  the boss becomes its owner. There is no way back to a local file, only an export.
- An online company is not played offline. Without a connection the launcher shows it as it was
  last seen, read-only.

## Versions

The launcher, the game and both servers each say their protocol version, as `omsi-net` does
(`PROTOCOL`). A launcher of another version is told to update instead of being refused without a
word. The company server keeps the `wire` version of `code.rs` for this.

## Privacy and moderation

- An online company keeps data of other people: who drove what, when, their names in the log.
  `docs/PRIVACY.md` gets a section for it; leaving a company removes the member from it (their
  trips stay, anonymous); `delete_my_data()` does the same for all their companies.
- Kicking and banning exist (`wire` `kick`); a report of a member to the company's owner and to
  us comes later.
- Back-ups per company, and the owner can restore yesterday's.

## Limits

- Players per world: `max_players` (16 by default, 64 at most), the star relays everything.
- Online companies per account: one (see "Members and roles"). Members per company: **open**.
- One world per company and map at a time.
- A world with nobody in it for N minutes stops; a company nobody has opened in N months is
  archived.

## What it costs: 100 companies

**Measured** (2026-11-01, this repository's dev machine: AMD Ryzen 7 5700X3D, 8 cores / 16
threads): `openomsi.exe --server` on Berlin-Spandau, the timetable, traffic 30, passengers on,
`radius = 0` (every tile loaded), no players, four minutes:

- loaded in about 10 s (1 GB after 10 s);
- memory steady at **1.1-1.2 GB** (private 1.17-1.35 GB);
- CPU **0.3 of one core** on average once loaded (43 CPU seconds in the last 140 s).

**Not measured**: a world with players in it. The traffic and the people are simulated around
each player, so that costs more; reckoned below at 1.5-2 GB and 0.5-1 core per world with a few
players. **To do**: measure with 4 and with 8 players before buying anything.

**The company servers** are cheap: 100 companies are a few MB each in Postgres, and one small
machine (2 cores, 4 GB) answers all of them. Some €10-30 a month, the database included.

**The worlds** cost what runs at the same moment, not how many companies there are. Reckoned for
the evening, the busiest hours:

| | Worlds at once | Players (≈3 a world) | Machines (8 cores, 64 GB) | A month |
| --- | --- | --- | --- | --- |
| Quiet | 10 (10% of the companies) | 30 | 1 | ≈ €60 |
| Expected | 30 (30%) | 90 | 1-2 | ≈ €60-140 |
| Busy | 60 (60%) | 180 | 2-3 | ≈ €130-210 |

- **One machine** of 8 cores / 16 threads and 64 GB holds about 20-30 worlds: by memory 30 at
  2 GB, by CPU 16-30 at 0.5-1 core. The prices are those of rented dedicated servers of that
  size (Hetzner's AX line, 2025: about €50-70 a month); **check them before buying**. Cloud
  machines by the hour cost more per world but follow the evening's peak.
- **The network** is no problem: a player gets about 20-40 KB/s (the other buses at 20 Hz, the
  traffic and people around them at 10 Hz), 180 players about 50 Mbit/s; rented servers come
  with tens of TB a month.
- **The disk**: every machine needs the maps of the companies whose worlds it runs. 100
  companies on perhaps 20 different maps are 50-150 GB.
- **In all**: about **€80-250 a month for 100 companies**, €1-2.50 a company. What moves it most
  is how many worlds run at once in the evening, then how heavy a world with players is.

**Before the costs matter - the licences** (**open**): a world server needs OMSI 2's own
content and the company's map installed on our machine. OMSI 2 is sold per player; a payware
map cannot be put on our server for companies whose owners bought it; and many freeware authors
do not allow their addons to be passed on (which `mods.rs` does). This is to be settled - with
Aerosoft/MR Software and with the authors, or by letting companies run their own world server -
before we rent anything. A company that runs its own world server costs us only the company
server.

## Phases

0. **The addon list in today's Play together**: the host's launcher makes it, the joining
   launcher compares. Useful at once, and the online company needs it as it is.
1. **The company server online**: `host.rs` as its own process on `hub-core`, Postgres, the
   membership tables (one company per account), the account's token instead of the code, the
   owner's clock. Driving as today: solo, or a member's game hosts the world. Most of the value
   for the least work.
2. **The company's package**: lines, depots and liveries with versions, read with `--content-zip`;
   `export_to_map` no longer used for online companies.
3. **The world server on demand**: `omsi --server` started and stopped by the company server, the
   company's `LivePlan` in it, the world as witness of the trips.
4. **The rest**: payware marks and stand-ins by kind, push instead of poll, back-ups, reports.

## Open points for Luc

- **The licences** of OMSI 2, payware maps and freeware addons on our world servers (see "What it
  costs").
- **Who runs the world servers**: we do (about €80-250 a month for 100 companies), companies run
  their own, or both.
- **Payware**: only for members who own the bus, a stand-in for the others - or another rule.
- **Members per company.**

Decided (2026-11-01): the owner sets the clock, real time or faster; one online company per
account; the addon list, in Play together too.

## Upstream (openOMSI)

- `--server` with `--company`, and a `LivePlan` that can change while the server runs.
- A content root (`--content-zip`) that wins over the map's own `TTData`.
- Stand-ins of the same kind and length as the bus they stand in for.
- `mods.rs` serving the whole `vehicles` list of a server, not only the host's own bus.
- The clock's speed of a running server set by the company server, not only by an admin in the
  chat.
