-- Omsi-Hub's plugin: the company's buses in the game. For a company's duty Omsi-Hub writes the
-- trips of the map's timetable the company's own buses run today, each with how late the
-- company's day has it (data/fleet.json: crates/hub-game/src/bridge.rs `Fleet`, worked out by
-- crates/hub-api/src/company/fleetgame.rs). This gives them to the game
-- (`omsi.timetable.set_ai_trips`): its AI runs those trips and no others - no bus of a line the
-- company does not run, none of a tour it dropped - each as late as the fleet map has it.
--
-- An openOMSI without `set_ai_trips`: the other timetable buses are taken off the road as they
-- come (`traffic.remove`), and the company's run on time. No file, another map or a drive of the
-- player's own: the map's whole timetable, as ever.
--
-- Each step in a call of its own, a step every second (openOMSI's 50 ms for one call, the game's
-- work inside counted: budget.lua; `step` below): the file read in one, the trips given to the
-- game in the next; the timetable's buses read in one, and those to go taken off a few in each
-- call after.

local budget = require("budget")

local M = {}

-- the file is looked at this often (s); Omsi-Hub writes it when the day's delays moved on
local EVERY = 5
-- a departure is the one asked for this close either side (s)
local NEAR = 45
local DAY = 86400

local wait = 0
local has = {}
-- the file's text last given to the game, and whether the game runs a list of ours
local seen
local given = false
-- (an openOMSI without set_ai_trips) the trips the company runs: "line|tour" -> departures
-- (true: every trip of the tour)
local allowed
-- the tour whose bus the player took over ("line|tour"): the AI no longer runs it
local off_key
-- the job under way, a step in each call (`step`)
local job = nil
-- at most this many buses taken off the road in one call, and trips or buses looked at
M.PORTION = 8
M.TRIPS = 250

M.NEEDS = { "files.read", "json.decode", "timetable.set_ai_trips", "timetable.buses", "traffic.remove", "map.info" }

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

-- whether this openOMSI can keep the company's buses alone on the road
function M.can()
  return has["files.read"] == true and has["json.decode"] == true
    and (has["timetable.set_ai_trips"] or (has["timetable.buses"] and has["traffic.remove"])) == true
end

local function key(line, tour)
  return tostring(line or ""):match("^%s*(.-)%s*$"):lower() .. "|" .. tostring(tour or ""):match("^%s*(.-)%s*$"):lower()
end

local function norm(path)
  return tostring(path or ""):gsub("\\", "/"):gsub("^%./", ""):match("^%s*(.-)%s*$"):lower()
end

-- the same map, however the game and the company name it (a global.cfg with or without the
-- game's folder before it)
local function same_map(game, company)
  local g, c = norm(game), norm(company)
  if g == "" or c == "" or g == c then return true end
  return g:sub(-#c - 1) == "/" .. c or c:sub(-#g - 1) == "/" .. g
end

-- the company's trips in the file's text (nil: none for this game), as the file has them: on a
-- large map thousands, each made anew in Lua would be a long call (Omsi-Hub writes them as
-- `set_ai_trips` takes them)
local function read(text)
  if not text or text == "" then return nil end
  local ok, f = pcall(omsi.json.decode, text)
  if not ok or type(f) ~= "table" or type(f.trips) ~= "table" then return nil end
  if has["map.info"] then
    local m = omsi.map.info()
    if type(m) == "table" and not same_map(m.path, f.map) then return nil end
  end
  return f.trips
end

-- the trips without the tour the player took over (a step of its own: a key for each trip)
local function without_off(trips)
  local out = {}
  for _, t in ipairs(trips) do
    if type(t) == "table" and key(t.line, t.tour) ~= off_key then out[#out + 1] = t end
  end
  return out
end

-- (an openOMSI without set_ai_trips) a portion of the trips into `allowed`: "line|tour" ->
-- departures (true: every trip of the tour); true when all are in
local function allow(job)
  local trips = job.trips
  local last = math.min(job.i + M.TRIPS - 1, #trips)
  for i = job.i, last do
    local t = trips[i]
    if type(t) == "table" then
      local k = key(t.line, t.tour)
      local departure = type(t.departure) == "number" and t.departure or nil
      if departure == nil then
        job.allowed[k] = true
      elseif job.allowed[k] ~= true then
        job.allowed[k] = job.allowed[k] or {}
        table.insert(job.allowed[k], departure)
      end
    end
  end
  job.i = last + 1
  return job.i > #trips
end

-- (an openOMSI without set_ai_trips) whether the timetable bus `b` runs a trip of the company's
local function runs(b)
  local a = allowed[key(b.line, b.tour)]
  if a == true then return true end
  if not a or type(b.departure) ~= "number" then return a ~= nil end
  for _, d in ipairs(a) do
    local dt = (b.departure - d) % DAY
    if dt < NEAR or dt > DAY - NEAR then return true end
  end
  return false
end

-- the next step of the job under way, alone in its call; nil when the job is done. The steps:
-- the file's trips read ("read") -> without a tour taken over ("off") -> given to the game
-- ("give"); in an openOMSI without set_ai_trips instead kept as `allowed` a portion at a time
-- ("allow") -> the timetable's buses read ("look") -> those to go found ("find") -> taken off the
-- road a few at a time ("remove")
local function step(job)
  if job.kind == "read" then
    local trips = read(job.text)
    if trips and off_key then return { kind = "off", trips = trips } end
    return { kind = "give", trips = trips }
  elseif job.kind == "off" then
    return { kind = "give", trips = without_off(job.trips) }
  elseif job.kind == "give" then
    if has["timetable.set_ai_trips"] then
      if job.trips or given then
        omsi.timetable.set_ai_trips(job.trips)
        given = job.trips ~= nil
      end
      return nil
    end
    allowed = nil
    if not job.trips then return nil end
    return { kind = "allow", trips = job.trips, i = 1, allowed = {} }
  elseif job.kind == "allow" then
    if not allow(job) then return job end
    allowed = job.allowed
    return { kind = "look" }
  elseif job.kind == "look" then
    return { kind = "find", buses = omsi.timetable.buses() or {}, i = 1, ids = {} }
  elseif job.kind == "find" then
    local last = math.min(job.i + M.TRIPS - 1, #job.buses)
    for i = job.i, last do
      local b = job.buses[i]
      if type(b) == "table" and b.id ~= nil and not runs(b) then job.ids[#job.ids + 1] = b.id end
    end
    job.i = last + 1
    if job.i <= #job.buses then return job end
    if #job.ids == 0 then return nil end
    return { kind = "remove", ids = job.ids }
  elseif job.kind == "remove" then
    -- (each is the game's work: its passengers get out)
    local n = 0
    while #job.ids > 0 and n < M.PORTION and not budget.over(15) do
      omsi.traffic.remove(table.remove(job.ids))
      n = n + 1
    end
    if #job.ids > 0 then return job end
    return nil
  end
  return nil
end

-- every second: the file looked at every few seconds; `company`: the game drives a company's
-- duty of Omsi-Hub's (in any other drive the map's whole timetable)
function M.tick(seconds, company)
  if not M.can() then return end
  wait = wait - seconds
  -- (a job under way: its next step)
  if job then
    job = step(job)
    return
  end
  if wait > 0 then return end
  wait = EVERY
  local text = company and omsi.files.read("fleet.json") or nil
  if text ~= seen then
    seen = text
    job = { kind = "read", text = text }
  elseif allowed then
    job = { kind = "look" }
  end
end

-- the player took over the bus of `line`, `tour` (takeover.lua): the AI keeps off that tour from
-- now on (the list given again at the next look)
function M.keep_off(line, tour)
  local k = key(line, tour)
  if off_key == k then return end
  off_key, seen, wait, job = k, nil, 0, nil
end

-- the plugin stops: the game runs its whole timetable again
function M.stop()
  if given and has["timetable.set_ai_trips"] then omsi.timetable.set_ai_trips(nil) end
  given, allowed, seen, off_key, job = false, nil, nil, nil, nil
end

return M
