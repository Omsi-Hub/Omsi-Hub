# What changed

Each version of the Omsi-Hub add-on, newest first. The release on GitHub links here.

## 0.1.0 (pre-release, for testing)

The first Omsi-Hub that is an add-on: its own app beside the openOMSI you have installed, instead
of a game of its own. It needs **openOMSI 0.2.22 or newer** and an original **OMSI 2**, and it
never changes their files.

### In it

- **Omsi-Hub is no longer open source**: from this version it is proprietary software, free to
  download and use (`LICENSE`); the code up to 19 October 2026 was MIT. The releases and the
  updates come from the public `Luc-nbr/Omsi-Hub-Releases`, and the libraries' licences travel
  with the app (`third-party-licenses.txt`).
- **The welcome** on the first start, as in the old Omsi-Hub: the language, the animations, your
  openOMSI folder and your OMSI 2 folder (it suggests the ones it finds, on Steam too, and checks
  openOMSI's version), your **account** - sign in, or continue locally without sync -, your driver
  and the way on (the tour, or straight to the start). Before the first drive, Omsi-Hub puts its
  plugin into openOMSI (Settings, "openOMSI and OMSI 2", switches it off or removes it).
- **The phone in openOMSI's navigator** (openOMSI 0.3.38 and newer): under the navigator's map a
  status line (the trip's line, where it goes, trip 3 of 8, on time or how late in the
  navigator's own colours) and five tabs, all working without F10:
  - **Ride**: the next stop large with when the bus gets there, the stop after it, passengers,
    speed and delay, and only the lines that apply - the bus ahead and how long to hold, a
    connection to wait for, a diversion, the control room's instruction waiting (one click to
    Messages), the company's punctuality coach - and the trip's progress.
  - **Stops**: the trip's stops, those passed, the next one and the rest, each late arrival in
    the time's colour, closed stops marked; during a break the next trip's.
  - **Duty**: starting the duty and signing off, the duty strip (the next break, relief or end
    counting down), today's changes, the duty's trips, the IBIS codes of the trip (and, where the
    game can type them, "Type the IBIS for me"), your week and, from the bus, a sick call, a duty
    swap or a day off, each asked again before it goes to the company, whose answer comes back
    as a notice.
  - **Messages**: the control room's messages, newest first, an instruction with Accept and
    Can't, the canned replies and the quick reports to control; a message is told read once
    shown.
  - **Bus**: fuel or charge (and when to go back to refuel), seated and standing passengers,
    the tickets sold and fares taken, collisions and repairs, the bus's open defects, the notes
    left for you and one for the next driver.
  As a trip ends its card (score, punctuality, comfort, passengers, fares), at a terminus with
  time to spare the break clock and as the duty ends its summary (trips, on time, score,
  passengers, km, fares - also kept in Omsi-Hub) take the Ride tab over. A short navigator gets
  one line (and the inbox its own panel), a light one dark text; Settings, "openOMSI and OMSI 2", switches the phone off, and
  the launcher says whether your openOMSI has it. With openOMSI 0.3.39 and newer the pause menu
  has the duty, a message to control and ending the duty, and closed stops are struck through on
  the navigator's map. An older openOMSI (0.3.37), or a navigator without its board, keeps the
  panels of before.
- **The phone in Omsi-Hub's look** (the plugin 1.14.0): its tabs are a row of buttons - the tab
  shown in Omsi-Hub's blue with its name, the others their icon and a count where something
  waits -, the next stop is a blue card with when the bus gets there as a pill and the trip's
  progress under it, the stats are tiles (the delay's in the time's colour), the duty's trips are
  rows with the line's yellow plate (the trip under way on a blue card, those done grey), each
  message is a card of its own (an instruction to answer on a blue one), and the rail, the bars,
  the checkboxes and the buttons that go on are Omsi-Hub's blue instead of yellow and grey.
- **The phone as the whole navigator** (the plugin 1.15.0): with an openOMSI that lets a panel
  fill the navigator's frame (openOMSI-org/openOMSI#3277), the phone is the navigator - the
  status line and the tabs at its top, the navigator's map with its next turn under them on the
  Ride tab, and the Stops, Duty, Inbox and Bus tabs the whole navigator for themselves. The
  navigator is moved, sized and put away as before. An openOMSI without it keeps the phone under
  the map.
- **Signing on with your pass** (the plugin 1.13.0): a company's duty is always signed on for in
  the game with your personnel number and code - there is no switch to leave it out - and a
  drive of your own never is; your pass is shown in Settings, "openOMSI and OMSI 2", and on the
  duty's start, and a driver without one gets one.
  In the phone a keypad takes the number, then the code (a wrong one is said and typed again),
  then the duty order's Accept, all without F10; an older openOMSI keeps the company panel's
  fields.
- **Panels you size yourself**: with an openOMSI that lets panels be resized, the board, the
  trip's card, the inbox, the on-time card, the duty strip and the company's panel are sized at
  their edges and corners, and keep that size in later drives; the board lists as many stops as
  its height holds. openOMSI's settings in Omsi-Hub gain the navigator's size, the navigator as a
  widget, its 3D buildings and its route arrows, and its corner keeps where you dragged it.
- **Windows blocking openOMSI** (Smart App Control, "os error 4551"): Omsi-Hub takes the
  "downloaded from the internet" mark off openOMSI's program and its libraries before it starts
  them, and where Windows still refuses it, says so and what can be done.
- **The depot in 3D**: the yard as a model to turn round and zoom into (and from straight above,
  2D), every bus in it as its own model in the livery it wears (baked once from the bus as the
  game draws it), a hundred and more of them smoothly. Build it up as you like: put each block of
  spaces, workshop bay, wash lane, diesel station and the offices where you want them on the
  yard's grid, turn and move them, pull them down, buy more land beside it. The graphics can be
  set lower for slower computers.
- **Your depot, built by you**: a company founded now starts with its land only and a building
  credit for a first depot (spent before its cash). Until it has a workshop its buses go to a
  garage outside, dearer and a day longer; until it has spaces they stand in the street. Every
  area grows - more wash lanes, up to six diesel pumps - and has things to add: lights and
  cameras, carports or a parking hall, a paint booth, an inspection lane, a tyre store, wash
  gantries, interior cleaning, an underbody wash, water recycling, a bulk tank, AdBlue, a canteen,
  a control room, a training room and a solar roof, each with what it costs and does. Companies
  of before keep the yard they rented.
- **The depot's look**: its walls', roofs', trims' and doors' own colours, and walls of steel
  cladding, brick or plaster.
- **The depot in its town**: streets with their pavements, worn road markings, zebra crossings,
  street lamps, trees and parked cars round it, the trading estate's halls next door and blocks
  of flats further out, their windows lit in the evening; the surfaces textured (asphalt, concrete,
  brick, the wash bay's tiles, the workshop's coated floor ...) with cracks, tyre tracks and oil
  where they are used; and the company's people at work - mechanics at the buses in the bays,
  cleaners in the wash lanes, drivers on their way to their buses -, each role a cartoon figure
  of its own (the driver, the mechanic, the cleaner, the service employee, the inspector, the
  researcher) that walks and, where it works, stands at ease.
- **A profile photo**, the same in the app and on the website: upload it in the account panel or
  on your service record (a click on the picture), or on omsihub.com; your friends, the
  leaderboard and the website show it.
- **The leaderboard gets your figures**: signed in, Omsi-Hub sends your driver's points, level,
  duties, time driven and kilometres whenever they change (after every drive).
- **One account, one driver**: signed in, your driver is your account's (the display name's), with
  the friends, the leaderboard and the cloud saves. Other drivers on the same computer play locally.
- **Your old saves** from the Omsi-Hub that came with its own game are offered on the first start:
  shown first, then copied (your companies, careers and settings). The old folder is left as it was.
- **The bus stop signs** on the maps - the drive's route map, the line editor and the company's
  fleet map -, as you choose them in the welcome or the settings: the German H (the default), the
  British roundel, the French, the Dutch, the Polish, the Russian or the Ukrainian sign. The route map of the
  drive's steps is the old one again: its stops with their times, dragged to move, the wheel or
  the + and - to zoom.
- **Omsi-Hub's panels in the game, your size and your place**: with F10 each one (the duty board
  beside the navigator, the company's panel, the duty strip, the on-time card, the control room's
  messages) shows a size row: smaller, larger (60 to 200 %), and back to how it came. Dragged to
  another place, a panel comes there again in your next drive. In an openOMSI with the navigator's
  board (plugin UI 3, in openOMSI after 0.3.36), the duty board is in the navigator again, as in
  the old Omsi-Hub, and is sized and moved with the navigator.
- **Blue** is Omsi-Hub's colour unless you choose another (Settings, Accent colour).
- **The drivers on this computer**, as in the old Omsi-Hub: "Who is driving today?" is the drive's
  first step again, with a tile for each driver (OMSI's own personnel files, `Drivers/<name>.odr`),
  a new one made in place and one deleted after asking. The game is started with the driver
  chosen (`--driver`), so it keeps their figures; the **service record** shows them with the
  level, the experience and the recent runs openOMSI wrote down.
- **Four ways to drive**: a work shift built from the timetable's real trips, a tour, a free drive,
  and the bus company. Omsi-Hub starts openOMSI with the map, the bus and the duty; the plugin
  shows the duty in the game and sends the trips back when you are done.
- **The bus company and RosterFlow**: buses, drivers, lines, the days' plans and the rosters, with
  control-room messages and instructions carried out in the game.
- **The editors**: the timetable editor (a map's lines and tours) and the depot editor (depot files
  of your own, for the IBIS and the destination displays). What they write goes into openOMSI's
  content folder or Omsi-Hub's own data folder, never into the OMSI 2 folder.
- **Online, optional**: an account (email or Google) for friends, the leaderboard and up to two
  cloud saves. Without one, everything works offline.
- **Windows, macOS (Apple silicon and Intel) and Linux** (an AppImage, or a `.deb` / `.rpm`).
- **English and six translations**: Dutch, German, French, Russian, Ukrainian and Polish, as the old
  Omsi-Hub had them.
- **openOMSI's own settings in Omsi-Hub** (Settings → openOMSI's settings): every key of
  `~/.openomsi/settings.cfg` that openOMSI's user guide names, 62 of them: the graphics (with
  Enhanced and Enhanced+, the view distance, the video memory, LED displays, rain and puddles),
  the mirrors, the screen and the navigator, the game's language, sound and the radio, driving and
  controls, and the passengers and the traffic. Omsi-Hub leaves the rest of the file as it is;
  "openOMSI's default" takes a key out again. Not while the game runs.
- **The installers in Omsi-Hub's look**: on Windows a welcome and a finish page with the logo, a
  picture beside every page's title, the licence, a choice of language (English, Dutch, German,
  French, Russian, Ukrainian) and a folder of its own in the Start menu; on macOS a disk image
  with its own window to drag Omsi-Hub into Applications; on Linux the packages' description.
- **The maps are drawn**, as the old Omsi-Hub drew them: the route map of the drive's steps, the
  company's duty map and its fleet map show the roads the game's own city map draws, read from the
  map's tiles (its splines' and its junctions' lanes), and the way each trip drives on its own
  lanes, with its stops.
- **Depot files read right**: a destination's strings were read a place off (a line OMSI's files
  do not have was taken for the stop it ends at), so its IBIS text was lost.
- **The bus in 3D** on the bus step, as the old Omsi-Hub drew it: the page's own 3D view reads
  every bus as the game reads it (its meshes, its textures, the livery and the bus options chosen,
  what its scripts show at rest), turned and zoomed with the mouse, with the views from the
  front, the side, the back and the corner, and the driver's and a passenger's view.
- **The buses' photos** on the fleet's, the dealer's and Casual's tiles and on the bus step, taken
  by the same 3D view (transparent, from the front corner) and kept until the bus changes. The
  photos the old Omsi-Hub took are still shown until then.
- **The livery studio**, as in the old Omsi-Hub: paint a bus in layers on the bus itself -
  colour areas, stripes, texts, pictures, own shapes and the brush, mirrored onto the other side -
  and save it into the game as a livery of its own (openOMSI's content folder, never the OMSI 2
  folder). The liveries begun before are opened from the same folder.
- **The line editor**, as in the old Omsi-Hub: a line of your own on any map - click its stops
  on the map and the way between them is found over the map's own lanes (the kerb side, lane
  changes), the way back made from the stops across the road, a leg dragged through a point;
  its destinations, its timetable and its kind of service; saved into the map's timetable so
  you and the timetable's buses drive it. For the bus company: its map, its lines, the estimate
  and "Confirm and pay". A line of the map's own opens to change it.
- **Multiplayer**, as in the old Omsi-Hub (on the start, "Multiplayer"): host your next duty
  and give friends its session code (at home and over the internet), join a friend's session by
  their code, or join a server that is always on - openOMSI's official one is listed first, and
  you can add your own. The game opens and joins the session itself (openOMSI's `--lan-host` and
  `--lan-join`), with the map, time and weather of the host; your driver's name is the one the
  others see.
- **Playing together**, as in the old Omsi-Hub: the boss shares the open company with a session
  code, friends join it in their own Omsi-Hub with the role the boss gives them and drive their
  duties in their own game. Over the LAN or a VPN, and over the internet through a Cloudflare
  tunnel whose address goes over openOMSI's own relay (no port forwarding needed).
- **The game's buses and traffic on the fleet map**: while you drive on the company's map, your
  bus and every bus of the company's tours the game drives are where the game has them, with the
  game's delay; the game's cars, trams and bicycles, the buses of the lines the company does not
  run and the other players' buses in a LAN game are drawn too (a switch in the panel hides the
  traffic). Needs an openOMSI with `omsi.traffic.list` and `omsi.timetable.buses` (0.2.22); the
  launcher shows whether yours has them.

### Not in it yet

- **What needs a newer openOMSI plugin API** than the official releases have: the bus's own display
  font, the IBIS typed in for you, closed stops struck through on the navigator, and the duty board
  docked in the navigator. The launcher shows which ones your openOMSI supports.

### Good to know

- **Updates.** This pre-release does not install updates itself yet: when a newer Omsi-Hub is out,
  the app says so and opens its page to download it.
- **macOS.** The app is not notarised: the first time, right-click it and choose Open.
- **Something wrong?** Tell us at [info@omsihub.com](mailto:info@omsihub.com) or in an issue, with
  the log: `omsi-hub.log` in Omsi-Hub's data folder (`%APPDATA%\Omsi-Hub` on Windows,
  `~/Library/Application Support/Omsi-Hub` on macOS, `~/.local/share/Omsi-Hub` on Linux).
