-- Omsi-Hub's plugin: the game's traffic for the company's fleet map - where the player's bus
-- is, every AI vehicle on the road, the timetable's buses with their line, tour and delay, and
-- the other players' buses in a LAN game - written every few seconds for Omsi-Hub, which puts
-- them on the map's roads (crates/hub-api/src/company_more/fleetmap.rs). Nothing is changed
-- in the game: it is all read.
--
-- The map's coordinates are the game's; with them comes one place given by its tile and the
-- metres in it (`ref`), so that Omsi-Hub can put them on its own reading of the map's tiles.
--
-- The file it shares with Omsi-Hub (crates/hub-game/src/bridge.rs `Traffic`):
--   data/traffic.json   plugin -> Omsi-Hub: the player, the vehicles, the timetable's buses
--
-- On a large map with much traffic that is a long read, a long list and a long text, and
-- openOMSI gives one call 50 ms with the game's work inside counted (budget.lua): so the game is
-- read in one call, its lists made into the file's text in the next, and the file written in
-- the one after.

local json = require("hubjson")

local M = {}

local EVERY = 2
local wait = 0
local has = {}

-- the functions this reads with, looked up in `omsi` itself (an openOMSI without game.has);
-- each is used where the game has it (an empty table is written as `{}`: Omsi-Hub reads it as
-- an empty list)
M.NEEDS = { "traffic.list", "timetable.buses", "position", "map.tile_at", "others", "map.info" }

local function fn(name)
  local t = omsi
  for part in name:gmatch("[^%.]+") do
    if type(t) ~= "table" then return nil end
    t = t[part]
  end
  if type(t) == "function" then return t end
  return nil
end

function M.setup(functions)
  has = {}
  for k, v in pairs(functions or {}) do has[k] = v end
  for _, n in ipairs(M.NEEDS) do has[n] = fn(n) ~= nil end
end

-- whether this openOMSI can tell Omsi-Hub anything of its traffic
function M.can()
  return has["files.write"] == true and (has["traffic.list"] or has["timetable.buses"] or has["position"]) == true
end

local function dm(x)
  if type(x) ~= "number" or x ~= x then return nil end
  return math.floor(x * 10 + 0.5) / 10
end

-- one place of the game's coordinates with its tile and the metres in it
local function reference(x, y)
  if has["map.tile_at"] and x and y then
    local tx, ty, lx, ly = omsi.map.tile_at(x, y)
    if tx then return { x = x, y = y, tile_x = tx, tile_y = ty, local_x = lx, local_y = ly } end
  end
  -- (an openOMSI without tile_at: the player's bus, where info has its tile)
  if fn("info") and has["position"] then
    local i = omsi.info()
    local px, py = omsi.position()
    if type(i) == "table" and px and i.tile_x and i.tile_pos_x then
      return { x = px, y = py, tile_x = i.tile_x, tile_y = i.tile_y, local_x = i.tile_pos_x, local_y = i.tile_pos_y }
    end
  end
  return nil
end

-- the map the game drives on (its global.cfg)
local function map_path()
  if has["map.info"] then
    local m = omsi.map.info()
    if type(m) == "table" and type(m.path) == "string" then return m.path end
  end
  if fn("info") then
    local i = omsi.info()
    if type(i) == "table" and type(i.map_path) == "string" then return i.map_path end
  end
  return ""
end

-- what the game has on the road now, as it gives it (the first step: only the game's reads)
function M.read(now, clock)
  local r = { at = now, clock = clock, map = map_path() }
  if has["position"] then
    local _
    r.x, r.y, _, r.heading = omsi.position()
    if r.x then
      r.speed = fn("speed") and omsi.speed() or 0
      r.info = fn("info") and omsi.info() or nil
    end
  end
  if has["traffic.list"] then r.traffic = omsi.traffic.list() end
  if has["timetable.buses"] then r.buses = omsi.timetable.buses() end
  if has["others"] then r.others = omsi.others(100000) end
  return r
end

-- what is on the road, as Omsi-Hub reads it, from what `read` read (the second step)
function M.build(r)
  local out = { at = r.at, clock = r.clock, map = r.map, vehicles = {}, buses = {}, players = {} }
  local first
  if r.x then
    local x, y = r.x, r.y
    out.player = { x = dm(x), y = dm(y), heading = dm(r.heading or 0), speed = dm(r.speed) }
    -- (its duty, its delay and its passengers, as the game has them)
    local i = r.info
    if type(i) == "table" then
      out.player.line, out.player.tour = tostring(i.line or ""), tostring(i.tour or "")
      out.player.delay, out.player.passengers = i.delay, i.passengers
    end
    first = { x, y }
  end
  -- (the AI vehicles, the timetable's buses kept for their own list below with their heading)
  local heading_of = {}
  for _, v in ipairs(r.traffic or {}) do
    if type(v) == "table" and v.x and v.y then
      if v.kind == "timetable_bus" then
        if v.id ~= nil then heading_of[v.id] = v.heading end
      elseif dm(v.x) and dm(v.y) then
        out.vehicles[#out.vehicles + 1] = { id = v.id, x = dm(v.x), y = dm(v.y), heading = dm(v.heading or 0) or 0, kind = tostring(v.kind or "car") }
      end
      first = first or { v.x, v.y }
    end
  end
  for _, b in ipairs(r.buses or {}) do
    if type(b) == "table" and b.x and b.y then
      out.buses[#out.buses + 1] = {
        id = b.id, x = dm(b.x), y = dm(b.y), heading = dm(heading_of[b.id] or 0),
        line = tostring(b.line or ""), tour = tostring(b.tour or ""), trip = b.trip, terminus = b.terminus or "",
        delay = b.delay, number = b.number and tostring(b.number) or "", at_stop = b.at_stop == true,
      }
      first = first or { b.x, b.y }
    end
  end
  -- (another player's bus in a LAN game)
  for _, o in ipairs(r.others or {}) do
    if type(o) == "table" and o.kind == "player" and o.x and o.y then
      out.players[#out.players + 1] = { name = o.name or "", x = dm(o.x), y = dm(o.y), heading = dm(o.heading or 0) }
    end
  end
  if first then out.ref = reference(first[1], first[2]) end
  return out
end

-- what is on the road now, all three steps at once (for a test)
function M.snapshot(now, clock)
  return M.build(M.read(now, clock))
end

-- (the step under way: what was read, the text to write)
local got, text = nil, nil

-- twice a second (main.lua): the file every few seconds, in three calls - the game read, the
-- text made, the file written
function M.tick(seconds, now, clock)
  if not M.can() then return end
  wait = wait - seconds
  if text then
    omsi.files.write("traffic.json", text)
    text = nil
    return
  end
  if got then
    -- (openOMSI's own encoder: the lists are lists, an empty one `[]`)
    text = json.fast(M.build(got))
    got = nil
    return
  end
  if wait > 0 then return end
  wait = EVERY
  got = M.read(now, clock)
end

return M
