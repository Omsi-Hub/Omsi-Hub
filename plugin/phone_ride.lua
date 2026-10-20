-- Omsi-Hub's plugin: the phone's Ride tab (nav.lua) - the drive now: the stop behind the bus and
-- the next ones, what comes after this trip, passengers, speed and delay, the company's parts.
-- Its modes: the trip's card as a trip ends, then at a terminus with time to spare the break
-- clock counting down to the next departure.

local board = require("board")
local card = require("card")
local company = require("company")
local P = require("phone_theme")

local M = {}

M.id = "ride"
M.title = "Ride"
M.icon = "navigation"
M.fit = { 4, 4 }

-- (the trip's card shows this long after the trip ended, game seconds)
local DONE_FOR = 20

-- passengers, km/h and the delay, side by side
local function stats(ctx)
  local th, tr = ctx.theme, ctx.tr
  local text, color = P.delay(ctx.delay, th, tr)
  return { type = "row", gap = 6, children = {
    P.stat(th, tr("Passengers"), tostring(ctx.passengers or 0), nil, ctx.can.box),
    P.stat(th, tr("km/h"), tostring(math.floor((ctx.speed or 0) + 0.5)), nil, ctx.can.box),
    P.stat(th, tr("Delay"), text, color, ctx.can.box),
  } }
end

function M.rows(ctx)
  local th, tr = ctx.theme, ctx.tr
  if not ctx.duty then
    return { P.text(th, ctx.at_work and tr("Not started yet") or tr("Not signed on"), "dim") }
  end
  local rows = board.rows(ctx.duty, ctx.trips, ctx.stops, ctx.clock, ctx.fit, not ctx.can.stops)
  -- (the status line has the badge, the terminus and the chip: the board's own first row goes)
  table.remove(rows, 1)
  rows[#rows + 1] = stats(ctx)
  if (ctx.room or 0) > 300 and ctx.hooks.parts then
    for _, x in ipairs(company.elements(ctx.hooks.parts(), ctx.assignment, ctx.duty, ctx.trips, ctx.clock, ctx.hooks.boarded and ctx.hooks.boarded() or nil, ctx.fit > 1)) do rows[#rows + 1] = x end
  end
  return rows
end

-- the trip just done (card.lua's record of it), while it is fresh
local function done(ctx)
  local last = card.last()
  if not (last and ctx.duty) then return nil end
  local age = ctx.clock - (last.clock or 0)
  if age < 0 or age > DONE_FOR then return nil end
  return last
end

-- the trip's card: how it was judged
local function trip_done(ctx, last)
  local th, tr = ctx.theme, ctx.tr
  local e = last.judged or {}
  local run = last.run or {}
  local out = {
    { type = "row", gap = 8, children = {
      { type = "icon", name = "sports_score", size = 18, color = th.ink },
      { type = "text", text = tr("Trip done"), size = 15, weight = "bold", color = th.ink, grow = true },
      { type = "text", text = tostring(e.score or 0) .. " / 100", size = 15, weight = "bold", color = th.ink, wrap = false },
    } },
    { type = "row", gap = 6, children = {
      P.stat(th, tr("Punctuality"), e.punctuality and (e.punctuality .. " %") or "–", nil, ctx.can.box),
      P.stat(th, tr("Comfort"), tostring(e.comfort or 0) .. " %", nil, ctx.can.box),
      P.stat(th, tr("Passengers"), tostring(run.passengers or 0), nil, ctx.can.box),
    } },
  }
  return out
end

-- the break clock: what is left until the next departure, large, and how much of the break is
-- gone; the trip that comes next
local function break_clock(ctx)
  local th, tr, d, trips, now = ctx.theme, ctx.tr, ctx.duty, ctx.trips, ctx.clock
  local this, after = trips[d.trip] or {}, trips[d.trip + 1] or {}
  local left = (after.departure or now) - now
  local length = math.max((after.departure or 0) - (this.arrival or 0), 1)
  local over = left < 0
  local ink = over and th.late or th.ink
  local out = {
    { type = "row", gap = 8, children = {
      { type = "icon", name = "schedule", size = 18, color = over and th.late or th.dim },
      { type = "text", text = over and tr("Break over") or tr("Break"), size = 13, weight = "bold", color = over and th.late or th.ink, grow = true },
      { type = "text", text = tr("until %{time}", { time = P.hhmm(after.departure) }), size = 12, color = th.dim, wrap = false },
    } },
    { type = "text", text = (over and "+" or "") .. P.mmss(math.abs(left)), size = 40, weight = "bold", align = "center", color = ink },
    { type = "bar", value = math.min(math.max(1 - left / length, 0), 1), height = 6, color = over and th.late or th.ink, background = th.track },
    { type = "row", gap = 7, children = {
      { type = "text", text = tr("Next %{time}", { time = P.hhmm(after.departure) }), size = 12.5, weight = "bold", color = th.ink },
      P.plate(th, after.line or "", tr),
      { type = "text", text = P.terminus(after.line, after.terminus), size = 12.5, color = th.ink, grow = true, wrap = false },
    } },
  }
  if (after.line or "") ~= (this.line or "") and (after.line or "") ~= "" then
    out[#out + 1] = P.text(th, tr("Continue on line %{line}, tour %{tour}", { line = after.line, tour = d.tour or "" }), "ink", 12, "bold")
  end
  return out
end

function M.mode(ctx)
  local last = done(ctx)
  if last then return trip_done(ctx, last) end
  if ctx.phase == "break" then return break_clock(ctx) end
  return nil
end

-- the slim phone: the break's minutes, else the next stop and when the bus is due there
function M.slim(ctx)
  local th, tr = ctx.theme, ctx.tr
  if not ctx.duty then return nil end
  if ctx.phase == "break" then
    local after = ctx.trips[ctx.duty.trip + 1] or {}
    return { type = "row", gap = 6, children = {
      { type = "icon", name = "schedule", size = 16, color = th.dim },
      { type = "text", text = tr("Break"), size = 12.5, weight = "bold", color = th.ink, grow = true },
      { type = "text", text = P.mmss((after.departure or ctx.clock) - ctx.clock), size = 14, weight = "bold", color = th.ink },
    } }
  end
  for _, s in ipairs(ctx.stops) do
    if s.stops ~= false and not s.passed then
      return { type = "row", gap = 6, children = {
        { type = "icon", name = "near_me", size = 16, color = th.dim },
        { type = "text", text = s.name or "", size = 12.5, weight = "bold", color = th.ink, grow = true, wrap = false },
        { type = "text", text = P.hhmm(s.arrival), size = 12.5, color = th.dim },
      } }
    end
  end
  return nil
end

return M
