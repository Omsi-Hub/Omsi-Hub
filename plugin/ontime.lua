-- Omsi-Hub's plugin: on time - what the bus company tells its driver on the way (Luc,
-- 2026-10-16): how the bus stands against the timetable as the company counts a trip, the gap
-- to the company's buses ahead and behind on a frequent line (and how long to hold when they
-- bunch), the protected connections at the stops ahead, and the diversions and roadworks of
-- the day - the closed stops struck through on the navigator, and a word before them.
--
-- Reported to the on-board computer (`facts`; Omsi-Hub's window shows it, nothing is drawn in
-- the game), switched off by the assignment (`ontime = false`: the launcher's setting). It is the
-- company's view of the delay (when the trip counts late, how far off that is), the headway, the
-- connections and the diversions; its alerts are notifications (notes.lua).
--
-- The files it shares with Omsi-Hub (crates/omsi-launcher-api/src/omsihub/ontime.rs):
--   data/live.json      plugin -> Omsi-Hub: where the bus is (the trip, its stops, the delay)
--   data/ontime.json    Omsi-Hub -> plugin: the brief (headway, connections, diversions, alerts)

local json = require("hubjson")
local notes = require("notes")

local M = {}

local WRITE_EVERY, READ_EVERY = 3, 3
-- a brief older than this (s) is of a game Omsi-Hub no longer looks after
local STALE = 60
local RED, NOW = "#FF5F57", "#F0B429"

-- the texts Omsi-Hub's brief sends as its alerts' templates (`notify` in ontime.rs): here so
-- that they are the plugin's texts, translated with the rest
local SENT = {
  "Hold %{secs} s at %{stop}: the bus ahead is close",
  "Close behind %{bus}: no time to hold",
  "Connection at %{stop}: wait max %{m} min",
  "Diversion ahead: %{stop} closed",
  "Roadworks on the line today: expect delays",
  "Diversion on the line today: expect delays",
  "Diversion on the line today",
}
M.SENT = SENT

local tr = function(t) return t end
local assignment, has = nil, {}
local brief, closed_sent = nil, nil
local write_wait, read_wait = 0, 0
local off = false

local function fn(t, name)
  return type(t) == "table" and type(t[name]) == "function"
end

function M.setup(translate, a, functions)
  tr, assignment, has = translate, a, functions or {}
  off = a ~= nil and a.ontime == false
end

-- on: a company's duty, not switched off
function M.on()
  return not off and assignment ~= nil and (assignment.duty or "") ~= ""
end

local function now()
  return fn(omsi.util, "now") and omsi.util.now() or 0
end

local function clock()
  local i = fn(omsi, "info") and omsi.info() or nil
  return type(i) == "table" and i.clock or 0
end

-- the company's view of a delay (`ontime::tone`): early from `early_from` s before, late from
-- `late_from` s behind
function M.tone(delay, early_from, late_from)
  if delay < -early_from then return "early" end
  if delay >= late_from then return "late" end
  if delay >= 60 then return "behind" end
  return "on_time"
end

-- --- the alerts ---------------------------------------------------------------------------

omsi.data.ontime_seen = omsi.data.ontime_seen or {}

-- The ONE way this card tells the driver something once: a notification (notes.lua). When the company's messages come (r67-messages: `post(company, to, priority,
-- template, args, action)`), the launcher posts them (ontime.rs `notify`) and this stays the
-- notification of a game that shows what it is sent - switch here to showing the posted message
-- instead.
local function alert(id, priority, template, args)
  if omsi.data.ontime_seen[id] then return end
  omsi.data.ontime_seen[id] = true
  local high = priority == "high"
  notes.say(tr(template, args), { title = tr("On time"), icon = high and "warning" or "schedule", color = high and RED or NOW, seconds = high and 10 or 6, sticky = high })
end
M.alert = alert

-- --- the files with Omsi-Hub ----------------------------------------------------------------

-- where the bus is, for Omsi-Hub
local function write_live(d)
  if not has["files.write"] then return end
  local stops = {}
  for _, s in ipairs(fn(omsi.duty, "stops") and omsi.duty.stops() or {}) do
    stops[#stops + 1] = { name = s.name or "", arrival = s.arrival or 0, departure = s.departure or 0, stops = s.stops ~= false }
  end
  local next = d.next_stop and (d.next_stop.number or 1) - 1 or #stops
  omsi.files.write("live.json", json.encode({
    issued = assignment and tonumber(assignment.issued) or 0, at = now(), clock = clock(),
    line = d.line or "", tour = d.tour or "", trip = d.trip or 0, trip_name = d.trip_name or "",
    next = next, at_stop = d.at_stop == true, delay = d.delay or 0, stops = stops,
  }))
end

-- Omsi-Hub's brief, when it is about this drive and this trip
local function read_brief(d)
  if not (has["files.read"] and has["json.decode"]) then return nil end
  local text = omsi.files.read("ontime.json")
  if not text then return nil end
  local ok, b = pcall(omsi.json.decode, text)
  if not ok or type(b) ~= "table" then return nil end
  local mine = assignment and tonumber(assignment.issued) or 0
  if (tonumber(b.issued) or 0) ~= mine or tonumber(b.trip) ~= d.trip then return nil end
  if now() > 0 and now() - (tonumber(b.at) or 0) > STALE then return nil end
  return b
end

-- the closed stops of the day to the navigator (where the game has it)
local function to_navigator(b)
  if not fn(omsi.duty, "set_closed_stops") then return end
  local list = b and b.closed or {}
  local key = table.concat(list, "|") .. "#" .. (b and b.via or "")
  if key == closed_sent then return end
  closed_sent = key
  omsi.duty.set_closed_stops(list, b and b.via or "")
end

-- --- the card ------------------------------------------------------------------------------

-- the card's facts for the duty `d` (`duty.get`) and the brief `b` (none yet: punctuality
-- alone, by the company's usual lines): plain data, a list with nothing in it left out
local function facts_of(d, b)
  b = b or {}
  local early_from, late_from = tonumber(b.early_from) or 60, tonumber(b.late_from) or 360
  -- (whole seconds: the game's delay is a float that moves every frame, and a part that changes
  -- every tick would be sent in every slot of the parts)
  local delay = tonumber(d.delay) or 0
  if delay == delay and delay ~= math.huge and delay ~= -math.huge then delay = math.tointeger(math.floor(delay + 0.5)) or delay end
  local out = { delay = delay, tone = M.tone(delay, early_from, late_from), early_from = early_from, late_from = late_from }
  -- the headway, where the line is frequent
  local h = b.headway
  if type(h) == "table" then
    local head = { planned = tonumber(h.planned) or 0 }
    if type(h.ahead) == "table" then head.ahead = { bus = h.ahead.bus, secs = h.ahead.secs } end
    if type(h.behind) == "table" then head.behind = { bus = h.behind.bus, secs = h.behind.secs } end
    local a = h.advice
    if type(a) == "table" and (a.kind == "hold" or a.kind == "close_behind") then head.advice = { kind = a.kind, secs = a.secs, bus = a.bus } end
    out.headway = head
  end
  -- the connections at the stops ahead
  local connections = {}
  for _, k in ipairs(type(b.connections) == "table" and b.connections or {}) do
    local st = type(k.status) == "table" and k.status or {}
    connections[#connections + 1] = {
      stop = k.stop, what = k.what, time = k.time, late = k.late or 0, onward = k.onward == true, max_hold = k.max_hold,
      status = { kind = st.kind, m = st.m, ["until"] = st["until"] },
    }
  end
  if #connections > 0 then out.connections = connections end
  -- the diversion ahead, and the day's closed stops
  if type(b.ahead) == "table" then out.ahead = { stop = b.ahead.stop, via = b.ahead.via or "" } end
  if type(b.closed) == "table" and #b.closed > 0 then out.closed = { table.unpack(b.closed) } end
  return out
end

-- the card's facts now (nil: it is off, there is no company's duty or no duty running)
function M.facts()
  if not (M.on() and fn(omsi.duty, "get")) then return nil end
  local d = omsi.duty.get()
  if not d then return nil end
  return facts_of(d, brief)
end

-- every second (main.lua): the files, the alerts, the closed stops
function M.tick(dt)
  if not (M.on() and fn(omsi.duty, "get")) then return end
  local d = omsi.duty.get()
  if not d then
    -- (the duty over: the navigator's closed stops cleared, where some were set)
    if closed_sent then to_navigator(nil) end
    brief = nil
    return
  end
  write_wait, read_wait = write_wait - (dt or 1), read_wait - (dt or 1)
  if write_wait <= 0 then
    write_wait = WRITE_EVERY
    write_live(d)
  end
  if read_wait <= 0 then
    read_wait = READ_EVERY
    brief = read_brief(d)
    if brief then
      to_navigator(brief)
      for _, a in ipairs(type(brief.alerts) == "table" and brief.alerts or {}) do
        if type(a) == "table" and a.id then alert(a.id, a.priority, a.template or "", a.args) end
      end
    end
  end
end

return M
