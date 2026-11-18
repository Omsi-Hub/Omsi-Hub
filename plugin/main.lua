-- Omsi-Hub in openOMSI: the duty the bus company assigned, its trips reported back to the
-- company, and everything the driver is told while driving sent to Omsi-Hub's on-board computer
-- (companion.lua) - through openOMSI's own plugin API, so that Omsi-Hub needs nothing of the
-- game's code. Nothing is drawn in the game while driving: the on-board computer, Omsi-Hub's
-- window over the game, shows it all; only Omsi-Hub's look for openOMSI's own screens (its
-- interface theme) is the plugin's in the game.
--
-- Installed and kept up to date by Omsi-Hub (crates/omsihub-plugin); a change by hand goes
-- with its next update. The files it shares with Omsi-Hub (see src/bridge.rs):
--   assignment.lua        Omsi-Hub -> plugin: the duty (its parts: legs.lua), the notices
--   texts.lua             Omsi-Hub -> plugin: the texts in the player's language
--   data/notices.json     Omsi-Hub -> plugin: notices that come while the game runs
--   data/messages.json    Omsi-Hub -> plugin: the control room's messages (messages.lua)
--   data/ontime.json      Omsi-Hub -> plugin: the on-time card's brief (ontime.lua)
--   data/fleet.json       Omsi-Hub -> plugin: the company's buses for the game's AI (fleet.lua)
--   data/hello.json       plugin -> Omsi-Hub: the handshake (what this openOMSI offers)
--   data/reports.jsonl    plugin -> Omsi-Hub: the duty taken, each trip done, the duty ended
--   data/replies.jsonl    plugin -> Omsi-Hub: the driver's answers and quick buttons
--   data/status.json      plugin -> Omsi-Hub: how the duty stands (for the dispatcher)
--   data/live.json        plugin -> Omsi-Hub: where the bus is (ontime.lua)
--   data/traffic.json     plugin -> Omsi-Hub: the game's traffic, for the fleet map (traffic.lua)
--   data/dash.json        plugin -> Omsi-Hub: the live dashboard of every drive (dash.lua)
--   data/pos.json         plugin -> Omsi-Hub: where the bus is, thirty times a second (dash.lua)
--   data/companion.json   Omsi-Hub -> plugin: the phone's commands (companion.lua, which sends
--                         the game to Omsi-Hub's companion server by UDP)
-- A game without the file functions gets the handshake and the reports in its log instead.

local VERSION = "1.19.0"
local BRIDGE = 1

local json = require("hubjson")
-- (each call into the plugin timed, and kept short: openOMSI stops a plugin whose one call runs
-- longer than 50 ms - budget.lua)
local budget = require("budget")
budget.install()
-- (the notifications, for the on-board computer: notes.lua)
local notes = require("notes")

-- The functions the add-on uses (src/compat.rs FEATURES; a test keeps the two lists the same),
-- looked up in the `omsi` table itself so that this works in an openOMSI without game.has.
local NEEDS = {
  "duty.get", "duty.trips", "files.append", "files.write", "duty.start",
  "files.read", "json.decode", "send",
  "duty.set_auto_ibis", "duty.type_ibis", "bus.set_display_font", "game.action",
  "duty.skip_to", "duty.finish", "duty.trip_stops", "audio.play",
  "duty.set_closed_stops", "traffic.list", "timetable.buses",
}

local function fn(name)
  local t = omsi
  for part in name:gmatch("[^%.]+") do
    if type(t) ~= "table" then return nil end
    t = t[part]
  end
  if type(t) == "function" then return t end
  return nil
end

local has = {}
for _, n in ipairs(NEEDS) do has[n] = fn(n) ~= nil end

-- the assignment Omsi-Hub wrote beside this file (none: the player drives on their own); one
-- older than half a day is of a game Omsi-Hub started before, not of this one
local assignment
do
  local ok, a = pcall(require, "assignment")
  if ok and type(a) == "table" then assignment = a end
  local now = fn("util.now") and omsi.util.now() or 0
  if assignment and now > 0 and (tonumber(assignment.issued) or 0) > 0 and now - assignment.issued > 12 * 3600 then assignment = nil end
end

-- the plugin's texts in the player's language: Omsi-Hub writes them beside the plugin
-- (`texts.lua`), and an older Omsi-Hub put them into the assignment
local texts = {}
do
  local ok, t = pcall(require, "texts")
  if ok and type(t) == "table" then texts = t end
end
local old_texts = assignment and type(assignment.texts) == "table" and assignment.texts or {}

-- a text in the player's language, with %{name} filled from `vars`
local function tr(text, vars)
  local t = texts[text] or old_texts[text] or text
  if vars then
    t = t:gsub("%%{(%w+)}", function(k)
      local v = vars[k]
      if v == nil then return "" end
      return tostring(v)
    end)
  end
  return t
end

local function issued()
  return assignment and tonumber(assignment.issued) or 0
end

-- the duty's parts, chained with duty.start (legs.lua)
local legs = require("legs")
legs.setup(assignment)
-- (duty.start in a call of its own: openOMSI's 50 ms for one call - start.lua)
local start = require("start")

-- one line to Omsi-Hub: into a file of the data folder, else into the game's log
local function send_out(file, tag, value, append)
  local text = json.encode(value)
  local done = false
  if append and has["files.append"] then
    done = omsi.files.append(file, text .. "\n") == true
  elseif not append and has["files.write"] then
    done = omsi.files.write(file, text) == true
  end
  if not done then omsi.log(tag .. text) end
  local port = assignment and tonumber(assignment.udp_port) or 0
  if append and port > 0 and has["send"] then omsi.send(port, text) end
  return done
end

local function hello()
  local h = {
    bridge = BRIDGE, plugin = VERSION, game = "", api = 1, platform = "", language = "",
    functions = has, issued = issued(),
  }
  if fn("game.version") then h.game = omsi.game.version() end
  if fn("game.api") then h.api = omsi.game.api() end
  if fn("game.platform") then h.platform = omsi.game.platform() end
  if fn("game.settings") then
    local s = omsi.game.settings()
    if type(s) == "table" and type(s.language) == "string" then h.language = s.language end
  end
  send_out("hello.json", "OMSIHUB-HELLO ", h, false)
end

local function now()
  if fn("util.now") then return omsi.util.now() end
  return 0
end

local function clock()
  if not fn("info") then return nil end
  local i = omsi.info()
  return type(i) == "table" and i.clock or nil
end

local function duty_now()
  if not has["duty.get"] then return nil end
  return omsi.duty.get()
end

local function report(kind, fields)
  local r = {
    kind = kind, issued = issued(), duty = assignment and assignment.duty or "",
    at = now(), clock = clock(),
  }
  for k, v in pairs(fields) do r[k] = v end
  return send_out("reports.jsonl", "OMSIHUB-REPORT ", r, true)
end

-- --- notices ----------------------------------------------------------------------------

omsi.data.seen = omsi.data.seen or {}

local function show_notice(n)
  if type(n) ~= "table" or not n.id or omsi.data.seen[n.id] then return end
  omsi.data.seen[n.id] = true
  local alert = n.kind == "alert"
  notes.say(n.text or "", { title = n.title, icon = alert and "warning" or "business", color = alert and "#FF5F57" or nil, seconds = 10, sticky = alert })
end

local function show_notices(list)
  if type(list) ~= "table" then return end
  for _, n in ipairs(list) do show_notice(n) end
end

local function live_notices()
  if not (has["files.read"] and has["json.decode"]) then return end
  local text = omsi.files.read("notices.json")
  if not text then return end
  show_notices(omsi.json.decode(text))
end

-- --- the company's duty -------------------------------------------------------------------

local state = { line = nil, tour = nil, last_trip = nil, started = false }

-- (a company's duty: a drive of the player's own has an assignment only for its driver)
local function company_duty()
  return assignment and (assignment.duty or "") ~= ""
end

local signon = require("signon")
local company = require("company")
company.setup(tr)

-- the board's company parts the driver switched on and off on the on-board computer (all on
-- until they chose; kept for the next drive)
omsi.data.board_parts = omsi.data.board_parts or {}
local function parts()
  return company.parts(omsi.data.board_parts)
end
-- (signing on, on the on-board computer: a duty with no pass is at work at once; the pass only
-- with a company duty)
signon.setup(company_duty() and assignment or nil)
-- (the on-time card: punctuality, headway, connections, diversions - `ontime.lua`, its own)
local ontime = require("ontime")
local fleet = require("fleet")
ontime.setup(tr, assignment, has)

-- the duty strip, the handover notes and the walk-around of a company's duty (duty_strip.lua):
-- its notes and defects go to Omsi-Hub as reports, its notices as notifications
local strip = require("duty_strip")
strip.setup(tr, company_duty() and assignment.crew or nil, function(kind, fields) report(kind, fields) end, function(icon, title, text)
  notes.say(text, { title = title, icon = icon, seconds = 8 })
end)

-- what the company's board says now (company.lua's facts, for the on-board computer): nil
-- without a company's duty
local function board_facts()
  if not company_duty() then return nil end
  local d = duty_now()
  local passengers = fn("bus.passengers") and omsi.bus.passengers() or nil
  local trip = require("trip")
  local line = assignment.line or ""
  local f = company.facts(parts(), assignment, {
    duty = d, trips = d and fn("duty.trips") and omsi.duty.trips() or {}, clock = clock() or 0,
    boarded = { trip.boarded_on_trip(), trip.boarded_total(), passengers },
    signed = signon.signed(), can_start = signon.at_work() and not d and has["duty.start"] == true and line ~= "",
  })
  -- (the control room's hold under way: `until`, `left`)
  f.hold = require("messages").hold()
  return f
end

-- the IBIS: typed by the game as each trip begins, unless Omsi-Hub says the driver types it -
-- and not before the driver accepted the duty order when signing on
local function ibis_rule()
  if not has["duty.set_auto_ibis"] then return end
  omsi.duty.set_auto_ibis(signon.at_work() and not (assignment and assignment.ibis == false))
end

local function say(text, title, icon, seconds)
  notes.say(text, { title = title, icon = icon, color = "#F58620", seconds = seconds or 10 })
end

-- a bus taken over halfway along its trip (the assignment's `take_over`: takeover.lua) - the
-- AI's bus of the tour, where it is
local takeover = require("takeover")
takeover.setup(assignment, function(text, title, icon) say(tr(text), tr(title), icon, 8) end)

-- the duty at the depot started; whether there was one to start
local function start_duty()
  if not (assignment and has["duty.start"]) or duty_now() or start.busy() then return false end
  local line, tour, trip, stop
  if legs.any() then
    line, tour, trip, stop = legs.first()
  else
    line, tour, trip, stop = assignment.line, assignment.tour, tonumber(assignment.trip) or 1, tonumber(assignment.stop) or 1
  end
  start.duty(line, tour, trip, stop, function(ok, why)
    if not ok then
      local text = tr("The duty could not be started: %{reason}", { reason = why or "" })
      notes.say(text, { icon = "warning", color = "#FF5F57", seconds = 8 })
    end
  end)
  return true
end

-- the part of the duty under way is done (legs.lua): the next one taken, or the duty ended
local function next_leg(what, line, tour, trip, stop)
  if what == "next" and has["duty.start"] then
    start.duty(line, tour, trip, stop, function(ok, why)
      if ok then
        local n = legs.current()
        say(tr("Part %{n} of %{of}: line %{line}, tour %{tour}", { n = n, of = legs.count(), line = line, tour = tour }), tr("Duty"), "route")
        if has["duty.type_ibis"] and not (assignment and assignment.ibis == false) then omsi.duty.type_ibis() end
      else
        say(tr("The duty could not be started: %{reason}", { reason = why or "" }), nil, "warning", 8)
      end
    end)
  elseif what == "done" then
    if has["duty.finish"] then omsi.duty.finish() end
    say(tr("Your duty is done: take the bus back to the depot"), tr("Duty"), "flag")
  end
end

-- --- the duty and its trips -------------------------------------------------------------------

-- what the duty's timetable says of trip `n` (from 1), and the bus and the map it is driven on
local function trip_facts(n)
  local f = { trip = n, trip_name = "", trips = 0, number = "", terminus = "", bus = "", map = "" }
  if has["duty.trips"] then
    local all = omsi.duty.trips()
    local t = all[n]
    f.trips = #all
    if t then
      f.trip_name, f.number, f.terminus = t.name or "", t.line or "", t.terminus or ""
      f.departure, f.arrival = t.departure, t.arrival
    end
  end
  if fn("bus.file") then f.bus = omsi.bus.file() or "" end
  if fn("map.info") then
    local m = omsi.map.info()
    if type(m) == "table" and type(m.path) == "string" then f.map = m.path end
  end
  return f
end

local function trip_report(line, tour, n, fields)
  local r = trip_facts(n)
  r.line, r.tour = line or "", tour or ""
  for k, v in pairs(fields) do r[k] = v end
  return report("trip", r)
end

-- --- the trip's card and the driver's record -----------------------------------------------

-- (an openOMSI with game.has also has trip_done; an older one is watched trip by trip)
local trip_events = fn("game.has") ~= nil

local trip = require("trip")
local card = require("card")
local watch = require("watch")
-- (the live dashboard in Omsi-Hub's window, on every drive: `dash.lua`)
local dash = require("dash")
dash.setup(trip, assignment, has)

-- the drive watch's notices: a notification for a while (notes.lua)
watch.setup(tr, function(icon, title, text)
  notes.say(text, { title = title, icon = icon, seconds = 5, color = icon == "traffic" and "#D93A30" or "#E8AA46" })
end)
if fn("game.settings") then
  local s = omsi.game.settings()
  -- (the units the player reads; an openOMSI without them says the API's own)
  if type(s) == "table" then watch.units = s.display_units or s.units end
end

local function standing()
  return not omsi.speed or omsi.speed() < 1
end

-- a trip that ended: its record into the report (Omsi-Hub books it), its card to the on-board
-- computer
local function trip_ended(run, fields)
  local c = trip.current()
  fields = fields or {}
  if run then fields.record = run end
  local saved = trip_report(state.line or (c and c.line_name), state.tour or (c and c.tour), fields.trip or (c and c.n), fields)
  if run then card.hold(run, saved, standing()) end
end

-- the trip under way, looked at a few times a second: another one began
local look = 0
local function follow_trip(dt)
  look = look - dt
  if look > 0 then return end
  look = 0.25
  local d = duty_now()
  if not d then
    trip.forget()
    return
  end
  local c = trip.current()
  if c and c.n == d.trip and c.line_name == d.line then return end
  -- (an older openOMSI without `trip_done`: `watch_trips` ends the trip and begins the next)
  if c and not c.done and not trip_events then return end
  trip.begin(d.trip, d.line, d.tour)
end

local function delay()
  local d = duty_now()
  return d and d.delay or nil
end


-- the control room's messages (messages.lua): the trip a short-turn or a relief ends reported
local messages = require("messages")
messages.setup(tr, { send = send_out, issued = issued, assignment = assignment, end_trip = function(how)
  local d = duty_now()
  if not d then return end
  local c = trip.current()
  local run = c and c.n == d.trip and trip.finish(false, delay(), watch) or nil
  trip_ended(run, { trip = d.trip, how = how, delay = delay() })
end })

-- the phone and tablet companion (companion.lua): what the phone shows, what it may do
local companion = require("companion")
-- (the duty order signed: further down)
local accepted
companion.setup({
  signon = signon, messages = messages, issued = issued,
  accept = function()
    if not (assignment and signon.accept(accepted)) then return false end
    ibis_rule()
    return true
  end,
  drive_free = function()
    state.started = true
    signon.skip()
    if duty_now() and has["duty.finish"] then omsi.duty.finish() end
    ibis_rule()
  end,
  -- (the on-board computer's commands)
  duty_start = start_duty,
  sign_off = function()
    -- (only at work: not while the driver still signs on or has the duty order to accept)
    if not signon.at_work() then return false end
    -- (the walk-around opens; with a pass the driver is back at the sign-on, and the game no
    -- longer types the IBIS)
    local ok = strip.sign_off()
    if signon.sign_off() then
      ibis_rule()
      ok = true
    end
    return ok
  end,
  part = function(id, on)
    if not company.LABELS[id] then return false end
    omsi.data.board_parts[id] = on == true
    -- (the duty strip is duty_strip.lua's, the only one: its switch is the board's "strip" part)
    if id == "strip" then strip.set_enabled(on == true) end
    return true
  end,
  strip = strip, notes = notes, card = card,
  -- (what each driving part reports of itself: the on-board computer shows them)
  facts = { strip = strip.facts, ontime = ontime.facts, board = board_facts, card = card.facts, notes = notes.list },
})

omsi.on("duty", function(line, tour)
  -- (a short-turn takes the same duty again at its next trip: no new duty)
  if messages.turned(line, tour) then return end
  if state.line then report("duty_end", { line = state.line, tour = state.tour }) end
  -- (the company's duty ended: the walk-around before the driver leaves the bus)
  if state.line and not line then strip.sign_off() end
  state.line, state.tour, state.last_trip = line, tour, nil
  if line then report("duty_start", { line = line, tour = tour }) end
  -- (a part that begins with a relief on the way: taken again from its relief stop)
  local from = legs.duty_began(line, tour)
  if from and has["duty.start"] then
    local _, l = legs.current()
    start.duty(line, tour, l.first, from)
  end
end)

omsi.on("trip_done", function(n, how, driving, comfort, tickets)
  local fields = { trip = n, how = how, driving = driving, comfort = comfort, tickets = tickets, delay = delay() }
  local c = trip.current()
  local run = c and c.n == n and trip.finish(how == "arrived", delay(), watch) or nil
  trip_ended(run, fields)
  next_leg(legs.trip_done(n))
end)

omsi.on("stop_arrive", function(_, _, number, d)
  trip.arrive(number, d)
  messages.arrive(number)
  -- (the part ends at a relief on the way: the trip ends here, the next part or the duty's end)
  local now = duty_now()
  if now and legs.relief_at(now.trip, number) then
    local c = trip.current()
    local run = c and c.n == now.trip and trip.finish(false, delay(), watch) or nil
    trip_ended(run, { trip = now.trip, how = "relieved", delay = delay() })
    say(tr("Your relief takes over here: your duty ends"), tr("Relief"), "group")
    next_leg(legs.advance())
  end
end)
omsi.on("stop_depart", function(_, _, number, d)
  trip.depart(number, d)
  companion.stop_depart(number, d)
end)
omsi.on("jolt", function() trip.jolt() end)
omsi.on("crash", function() trip.crash() end)
omsi.on("passenger_board", function(count) trip.boarded(count) end)
omsi.on("light_ahead", function(aspect) watch.light_ahead(aspect) end)
omsi.on("red_light", function(kmh) watch.red_light(kmh) end)

-- (the object editor once, as soon as the world is there: the frames come with it)
local editor_opened = false

-- the game busy - a frame long after the one before (the world just loaded, or the game
-- hitched): what can wait (the phone companion, the fleet map's traffic) waits until the frames
-- have come smoothly for a few seconds. openOMSI stops a plugin whose one call runs longer than
-- 50 ms, and while the game still loads beside it every call of the plugin takes longer. (In
-- real time: the game gives a frame at most a tenth of a second of its own.)
local CALM = 3
local last_frame, busy_at = now(), nil
local function calm()
  return not busy_at or now() - busy_at >= CALM
end

omsi.on("frame", function(dt)
  local t = now()
  if t - last_frame > 0.2 then busy_at = t end
  last_frame = t
  if assignment and assignment.editor and not editor_opened then
    editor_opened = true
    if has["game.action"] then omsi.game.action("object_editor") end
  end
  if fn("has_vehicle") and not omsi.has_vehicle() then return end
  takeover.frame(dt)
  trip.frame(dt)
  watch.frame(dt)
  strip.frame(dt)
  follow_trip(dt)
  card.frame(dt, standing())
  dash.frame(dt)
end)

local function watch_trips()
  if trip_events then return end
  local d = duty_now()
  if not d then return end
  if state.last_trip and d.trip > state.last_trip then
    local c = trip.current()
    local run = c and c.n == state.last_trip and trip.moved_on(watch) or nil
    trip_ended(run, { trip = state.last_trip, how = "arrived", delay = d.delay })
    trip.begin(d.trip, d.line, d.tour)
    local done = state.last_trip
    state.last_trip = d.trip
    next_leg(legs.trip_done(done))
    return
  end
  state.last_trip = d.trip
end

-- the display font chosen for the drive's bus, on that bus
local function display_font()
  local font = assignment and assignment.display_font or ""
  if font == "" or not has["bus.set_display_font"] then return end
  local file = fn("bus.file") and omsi.bus.file() or ""
  local want = (assignment.bus or ""):gsub("\\", "/"):lower()
  if want ~= "" and file:gsub("\\", "/"):lower() ~= want then return end
  omsi.bus.set_display_font(font)
end

-- a free drive along one route of a line: nobody keeps the time, the route is said once
local free_said = false
local function free_drive()
  local route = assignment and assignment.free_line or ""
  if free_said or route == "" then return end
  free_said = true
  say(tr("Free drive on line %{line}: %{route}", { line = assignment.line or "", route = route }), tr("Free drive"), "alt_route", 12)
end

omsi.on("vehicle", function(name)
  companion.vehicle()
  if name then display_font() end
  if name then free_drive() end
  -- (the notices once the player is in the bus and sees them)
  if name and assignment then show_notices(assignment.notices) end
  if name and signon.at_work() then strip.signed_on() end
  if name then ibis_rule() end
  if name and assignment and assignment.auto_start and signon.at_work() and not state.started then
    state.started = true
    start_duty()
  end
end)

-- signed on and the duty accepted (on the on-board computer, or on the phone): taken now where
-- it was to start by itself
accepted = function()
  if assignment.auto_start and not state.started then
    state.started = true
    start_duty()
  end
  -- (the duty's codes go to the IBIS as the order is accepted; the bus's notes are shown)
  strip.signed_on()
  ibis_rule()
  if has["duty.type_ibis"] and assignment.ibis ~= false then omsi.duty.type_ibis() end
end

-- signing on needs the on-board computer (Omsi-Hub's window, which Omsi-Hub's companion server
-- says is there: companion.lua's `device`). Where none has been seen for SIGN_ON_WAIT seconds
-- of a smoothly running game at the sign-on - the window cannot open (Wayland), or Omsi-Hub is
-- not running - nobody could sign on: the driver is at work, as with sign-on switched off. (A
-- window hidden behind a game in exclusive full screen still asks for its screens: that is not
-- noticed here.) Only before the driver first got past the sign-on.
local SIGN_ON_WAIT = 20
local no_device = 0
omsi.every(1, function()
  if not calm() or not signon.waiting() or companion.device() then
    no_device = 0
    return
  end
  no_device = no_device + 1
  if no_device < SIGN_ON_WAIT then return end
  no_device = 0
  signon.skip()
  omsi.log("[omsihub] no on-board computer seen in " .. SIGN_ON_WAIT .. " s: at work without signing on")
  -- (in the bus: as when the order is accepted; not yet: the bus's arrival does it)
  if not fn("has_vehicle") or omsi.has_vehicle() then accepted() else ibis_rule() end
end, "sign-on without a device")

omsi.on("stop", function()
  fleet.stop()
end)

-- service control's messages for the duty (`company.lua`), each once, when they are due
omsi.data.dispatched = omsi.data.dispatched or {}
local function dispatch()
  if not (company_duty() and parts().dispatch and signon.at_work() and duty_now()) then return end
  local c = clock()
  if not c then return end
  -- (said once for this assignment: a drive started again does not say them twice)
  local seen = omsi.data.dispatched
  if seen.issued ~= issued() then
    seen = { issued = issued() }
    omsi.data.dispatched = seen
  end
  for _, m in ipairs(company.messages(assignment, math.floor(c / 60), seen)) do
    seen[m.id] = true
    notes.say(m.text, { title = m.title, icon = "campaign", color = "#F58620", seconds = 10 })
  end
end

-- (the board's second: the control room's holds and instructions, the duty strip's relief, service
-- control's messages, the notifications' time)
omsi.every(1, function()
  messages.tick()
  strip.update(company_duty() and signon.at_work())
  dispatch()
  notes.tick(1)
end, "board")
-- (the on-time card and the live dashboard each in calls of their own)
omsi.every(1, function()
  if signon.at_work() then ontime.tick(1) end
end, "ontime")
omsi.every(0.5, function()
  if not fn("has_vehicle") or omsi.has_vehicle() then dash.tick(0.5) end
end, "dash")

omsi.every(2, watch_trips, "trips")
-- (in a company's duty only the company's own buses on the road, where its fleet map has them;
-- not held back while the game loads: the other buses are kept off before they are seen)
fleet.setup(has)
omsi.every(1, function()
  -- (the tour whose bus the player took over: the AI's no more)
  local line, tour = takeover.taken()
  if line then fleet.keep_off(line, tour) end
  fleet.tick(1, company_duty())
end, "fleet")
-- (the game's traffic for the fleet map: whatever the player drives)
local traffic = require("traffic")
traffic.setup(has)
-- (twice a second: the game read, the file's text made and the file written, each in a call)
omsi.every(0.5, function() if calm() then traffic.tick(0.5, now(), clock()) end end, "traffic")
omsi.every(5, live_notices, "notices")
omsi.every(1, companion.alive, "companion alive")
omsi.every(0.25, function() if calm() then companion.look() end end, "companion look")
omsi.every(0.25, function() if calm() then companion.tick(0.25) end end, "companion")
omsi.every(0.1, function() if calm() then companion.screen() end end, "companion screen")
omsi.every(2, messages.read, "messages")
omsi.every(5, messages.status, "status")

-- Omsi-Hub's look for openOMSI's own screens (theme.json beside this file: the pause menu's
-- panels of glass, the loading screen with the logo as its bar, Omsi-Hub's blue), put in force
-- once - the first time an openOMSI that knows interface themes (`ui.set_theme`) runs the
-- plugin, and only while the player has openOMSI's own look; what they choose in Options ›
-- Interface after that stays
local function offer_theme()
  if omsi.data.theme_offered or not (fn("ui.set_theme") and fn("ui.theme")) then return end
  omsi.data.theme_offered = true
  if omsi.ui.theme() ~= "" then return end
  local ok, why = omsi.ui.set_theme("omsihub")
  if not ok then omsi.log("[omsihub] the Omsi-Hub look is not there: " .. tostring(why)) end
end

hello()
offer_theme()
