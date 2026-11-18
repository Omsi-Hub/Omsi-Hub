-- Omsi-Hub's plugin: the trip's card, as Omsi-Hub's own game showed it at the end of a trip
-- (crates/omsi-app/src/trip_report.rs): the line and where it went, the stops served and how
-- many of them early, on time and late, the stop furthest off the timetable and the average,
-- the driving time, the distance, the jolts, the collisions and the passengers, how the trip is
-- judged (`career.evaluate`: a grade, its parts, the experience it earned, the fines) and each
-- stop with when it was due and when the bus came and left. Reported to the on-board computer
-- (`facts`; Omsi-Hub's window shows it over its map, nothing is drawn in the game): it comes when
-- the bus stands and stays some seconds of the game running, unless the driver closes it first
-- (the companion's `card_close`).

local career = require("career")

local M = {}

local SHOW_FOR = 15

local shown, waiting = nil, nil

-- how far off the timetable the stop `k` of `visits` counts (seconds; nil: it was not served)
local function visit_delay(visits, k)
  local v = visits[k]
  local last = k == #visits
  local arrival = v.arrived and (v.arrived - v.arr) or nil
  local departure = (not last and v.left) and (v.left - v.dep) or nil
  local decides = arrival and career.delay(arrival, departure) or nil
  if arrival then
    local j = career.judged(arrival, departure)
    if j == "early" then decides = departure or arrival elseif j == "late" then decides = arrival end
  end
  return decides
end

local function show(run, saved)
  shown = { run = run, judged = career.evaluate(run), saved = saved, left = SHOW_FOR }
end

-- a trip's card: now when the bus stands, else when it next does
function M.hold(run, saved, standing)
  if standing then
    if waiting then show(waiting[1], waiting[2]) waiting = nil end
    show(run, saved)
  else
    if waiting then show(waiting[1], waiting[2]) end
    waiting = { run, saved }
  end
end

-- the shown card as plain data: the trip's record (`run`, each stop with how far off it counted),
-- how it is judged (`career.evaluate`: grade, stars, score, parts, experience) and `open`; nil
-- when no card is shown. Numbers are whole; the stops are cut to the first 60 (`run.more.visits`
-- counts the rest), and companion.lua cuts a card still too long for a datagram the same way.
local MOST_STOPS = 60

-- (whole seconds, metres and the like: a float would take 20 bytes a number in the datagram)
local function whole(x)
  if type(x) ~= "number" or x ~= x or x == math.huge or x == -math.huge then return x end
  return math.tointeger(math.floor(x + 0.5)) or x
end

function M.facts()
  if not shown then return nil end
  local run = {}
  for k, v in pairs(shown.run) do
    if k ~= "visits" and k ~= "offences" then run[k] = whole(v) end
  end
  local visits = shown.run.visits or {}
  if #visits > 0 then
    run.visits = {}
    for k = 1, math.min(#visits, MOST_STOPS) do
      local v = visits[k]
      run.visits[k] = { name = v.name, arr = whole(v.arr), dep = whole(v.dep), arrived = whole(v.arrived), left = whole(v.left), delay = whole(visit_delay(visits, k)) }
    end
    if #visits > MOST_STOPS then run.more = { visits = #visits - MOST_STOPS } end
  end
  local offences = shown.run.offences
  if type(offences) == "table" and #offences > 0 then
    run.offences = {}
    for k, o in ipairs(offences) do
      -- (the seconds a light had been red keep their tenths)
      run.offences[k] = { kind = o.kind, at = whole(o.at), speed = whole(o.speed), limit = whole(o.limit), red_for = o.red_for, fine = whole(o.fine) }
    end
  end
  local out = { open = true, run = run, saved = shown.saved == true }
  for k, v in pairs(shown.judged) do out[k] = v end
  return out
end

function M.close()
  if not shown then return false end
  shown = nil
  return true
end

-- every frame: the time the card has left, and a card waiting for the bus to stand
function M.frame(dt, standing)
  if standing and waiting then
    show(waiting[1], waiting[2])
    waiting = nil
  end
  if shown then
    shown.left = shown.left - dt
    if shown.left <= 0 then return M.close() end
  end
end

return M
