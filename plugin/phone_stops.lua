-- Omsi-Hub's plugin: the phone's Stops tab (nav.lua) - the trip's stops on a rail: the one
-- behind the bus, the one it heads for and those to come, each with its time and, off the
-- timetable, when the bus will be there (in the time's colour); the stops closed today tagged.
-- As many as the room under the map has room for, and how many more to the terminus. During
-- the break the next trip's stops.
--
-- The rail is openOMSI's `stops` element (ui version 4). An older openOMSI, and the navigator's
-- light card (the rail writes its names in light ink), get a table in its place.

local P = require("phone_theme")

local M = {}

M.id = "stops"
M.title = "Stops"
M.icon = "departure_board"
M.fit = { 8, 64 }

-- (the most stops one rail takes)
local MOST = 128

-- the stops of the trip the bus calls at (the closed ones too: tagged) and which of them it
-- heads for: the list, and that one's place in it (nil: none left). `list`: these stops
-- (`duty.stops` as the game gives them) in place of the trip's under way.
function M.served(ctx, list)
  local out = {}
  local d = ctx.duty or {}
  local number = type(d.next_stop) == "table" and tonumber(d.next_stop.number) or nil
  local by_number, first = nil, nil
  for _, s in ipairs(list or ctx.stops or {}) do
    if s.stops ~= false then
      out[#out + 1] = s
      if not s.closed then
        if number and s.number == number and not by_number then by_number = #out end
        if not s.passed and not first then first = #out end
      end
    end
  end
  return out, by_number or first
end

-- when the bus will be at stop `s`, off the timetable (nil: on time), and in which colour
function M.expected(ctx, s)
  local delay = ctx.delay or 0
  if ctx.phase ~= "running" or P.punctuality(delay) == "on_time" then return nil end
  return P.hhmm((s.arrival or 0) + delay), P.delay_color(delay, ctx.theme)
end

-- the rail, or the table where the game draws no rail or draws it unreadable
local function rail(ctx, list)
  local th = ctx.theme
  if ctx.can.stops and not th.light then
    return { type = "stops", stops = list, color = th.accent, size = 13 }
  end
  local rows = {}
  for _, s in ipairs(list) do
    local ink = s.state == "done" and th.faint or (s.state == "now" and th.ink or th.dim)
    local name = s.name .. (s.tag ~= "" and ("  · " .. s.tag) or "")
    local mark = s.state == "done" and "✓" or (s.state == "now" and "▶" or "")
    if ctx.can.box then
      rows[#rows + 1] = { { text = mark, color = ink }, { text = name, color = ink }, { text = s.time, color = ink }, { text = s.expected, color = s.expected_color or ink } }
    else
      -- (an openOMSI before coloured cells writes a cell's table as nothing: plain texts there)
      rows[#rows + 1] = { mark, name, s.time, s.expected }
    end
  end
  return { type = "table", columns = {}, widths = { 0.08, 0.6, 0.16, 0.16 }, aligns = { "center", "left", "right", "right" }, rows = rows, size = 13, color = th.ink }
end

function M.rows(ctx)
  local th, tr = ctx.theme, ctx.tr
  if not ctx.duty then return { P.text(th, tr("No trip under way"), "dim") } end
  if ctx.phase == "finished" then return { P.text(th, tr("Duty finished"), "dim") } end
  local out = {}
  local served, now
  local terminus = ctx.terminus
  if ctx.phase == "break" then
    -- (the break: the next trip's stops, all still to come)
    local trip = ctx.trip or {}
    served = M.served(ctx, ctx.has("duty.trip_stops") and omsi.duty.trip_stops(ctx.k) or {})
    out[1] = P.text(th, tr("Next %{time}", { time = P.hhmm(trip.departure) }), "dim", 12, "medium")
  else
    served, now = M.served(ctx)
  end
  if #served == 0 then
    out[#out + 1] = P.text(th, tr("No trip under way"), "dim")
    return out
  end
  -- (the stop behind the bus, the one it heads for and the ones after it, as many as fit)
  local at = now or (ctx.phase == "break" and 1 or #served + 1)
  local from = math.max(math.min(at - 1, #served), 1)
  local to = math.min(from + math.max(ctx.fit or M.fit[1], 2) - 1, #served, from + MOST - 1)
  local list = {}
  for j = from, to do
    local s = served[j]
    local state = ctx.phase == "break" and "ahead" or (j < at and "done" or (j == at and "now" or "ahead"))
    local expected, color = nil, nil
    -- (none for the stop the bus stands at)
    if state == "ahead" or (state == "now" and not ctx.duty.at_stop) then expected, color = M.expected(ctx, s) end
    local tag = ""
    if s.closed then tag = tr("closed") elseif j == #served then tag = tr("terminus") end
    list[#list + 1] = { name = s.name or "", time = P.hhmm(s.arrival), expected = expected or "", expected_color = color, state = state, tag = tag }
  end
  out[#out + 1] = rail(ctx, list)
  if to < #served then
    out[#out + 1] = P.text(th, tr("and %{n} more to %{terminus}", { n = #served - to, terminus = terminus }), "faint", 11.5, "medium")
  end
  return out
end

return M
