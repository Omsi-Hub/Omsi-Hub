-- Omsi-Hub's plugin: the phone's summaries (the Ride tab's modes, phone_ride.lua) - as a trip
-- ends its card for a while: the score and grade the company gives it (career.lua, through
-- card.lua's record of the trip), punctuality, comfort, the passengers and the fares taken on
-- it; and as the duty ends the duty's: its trips, the stops served on time, the average score
-- and the grade it comes to, the passengers, the kilometres and the fares. The duty's totals go
-- with the duty-end report too (`fields`, main.lua).
--
-- main.lua tells it each trip done (`add`, with card.last()), each duty begun (`begin`) and
-- ended (`fields`). The fares are the game's tickets money (`bus.sales`) taken between them.

local career = require("career")
local company = require("company")
local strip = require("duty_strip")
local P = require("phone_theme")

local M = {}

-- (a trip's card shows this long after the trip ended, game seconds)
local SHOW_FOR = 20

-- the duty's trips done: { run, judged, clock, fares, tickets } each (card.last()'s and what
-- was taken on it); where the part under way began; the tickets and money at the last trip's
-- end; whether a duty ended and its summary was put away
local trips = {}
local first = 1
local mark = nil
local ended = false
local dismissed = false

-- the tickets sold and the money taken so far (nil: the game does not say)
local function sales()
  if not (omsi.bus and type(omsi.bus.sales) == "function") then return nil end
  local tickets, money = omsi.bus.sales()
  if not tonumber(money) then return nil end
  return tonumber(tickets) or 0, tonumber(money)
end

-- an amount of money as the company writes it (its symbol and decimals; two without one)
function M.money(assignment, amount)
  local fare = type(assignment) == "table" and assignment.fare or nil
  if type(fare) == "table" then return company.money(amount, fare) end
  return string.format("%.2f", amount)
end

-- a duty begins (`continued`: the next part of the same duty - its trips stay counted)
function M.begin(continued)
  if not continued then trips, dismissed = {}, false end
  first, ended = #trips + 1, false
  local tickets, money = sales()
  mark = money and { tickets, money } or nil
end

-- a trip done: `last` card.lua's record of it ({ run, judged, saved, clock })
function M.add(last)
  if type(last) ~= "table" or type(last.run) ~= "table" then return end
  local r = { run = last.run, judged = last.judged or career.evaluate(last.run), clock = last.clock or 0 }
  local tickets, money = sales()
  if money and mark then r.tickets, r.fares = math.max(tickets - mark[1], 0), money - mark[2] end
  if money then mark = { tickets, money } end
  trips[#trips + 1] = r
end

-- the totals of the trips from `from` on
local function totals(from)
  local n, score, passengers, metres, stops, on_time, fares = 0, 0, 0, 0, 0, 0, nil
  for k = from, #trips do
    local r = trips[k]
    local run, e = r.run, r.judged or {}
    n = n + 1
    score = score + (e.score or 0)
    passengers = passengers + (tonumber(run.passengers) or 0)
    metres = metres + (tonumber(run.metres) or 0)
    if not run.free and (tonumber(run.stops) or 0) > 0 then
      stops = stops + run.stops
      on_time = on_time + math.max(run.stops - (run.early or 0) - (run.late or 0), 0)
    end
    if r.fares then fares = (fares or 0) + r.fares end
  end
  return {
    trips = n, on_time = stops > 0 and math.tointeger(career.round(100 * on_time / stops)) or nil,
    score = n > 0 and math.tointeger(career.round(score / n)) or nil,
    passengers = math.tointeger(passengers) or passengers, km = career.round(metres / 100) / 10,
    fares = fares and career.round(fares * 100) / 100 or nil,
  }
end

-- the duty (or its part) ended: the duty-end report's `f` with the part's totals (trips,
-- on_time %, average score, passengers, km, fares) - the whole duty's at its end (`whole`), as
-- its summary shows them; its summary comes up
function M.fields(f, whole)
  local t = totals(whole and 1 or first)
  for _, k in ipairs({ "trips", "on_time", "score", "passengers", "km", "fares" }) do f[k] = t[k] end
  ended = true
  return f
end

-- the duty's summary put away (its close button)
function M.dismiss()
  dismissed = true
end

-- whether the duty has ended (until the next one begins)
function M.ended()
  return ended
end

-- --- the cards --------------------------------------------------------------------------------

-- the grade's five stars, those earned in the ink
local function stars(th, n)
  local row = { type = "row", gap = 1, grow = true, children = {} }
  for i = 1, 5 do row.children[i] = { type = "icon", name = "star", size = 13, color = i <= (n or 0) and th.ink or th.faint } end
  return row
end

-- a grade as a badge: neutral, the grade's word is enough
local function grade(ctx, word)
  return { type = "badge", text = ctx.tr(word), color = ctx.theme.chip, text_color = ctx.theme.ink, weight = "bold" }
end

local function trip_card(ctx, r)
  local th, tr = ctx.theme, ctx.tr
  local e, run = r.judged or {}, r.run
  local out = {
    { type = "row", gap = 8, children = {
      { type = "icon", name = "sports_score", size = 18, color = th.ink },
      { type = "text", text = tr("Trip done"), size = 15, weight = "bold", color = th.ink, grow = true, wrap = false },
      { type = "text", text = tostring(e.score or 0) .. " / 100", size = 15, weight = "bold", color = th.ink, wrap = false },
    } },
    { type = "row", gap = 8, children = {
      grade(ctx, e.grade or "Fair"),
      stars(th, e.stars),
      { type = "text", text = tr("+%{n} XP", { n = e.xp or 0 }), size = 12.5, weight = "medium", color = th.dim, wrap = false },
    } },
    { type = "row", gap = 6, children = {
      P.stat(th, tr("Punctuality"), e.punctuality and (e.punctuality .. " %") or "–", nil, ctx.can.box),
      P.stat(th, tr("Comfort"), tostring(e.comfort or 0) .. " %", nil, ctx.can.box),
      P.stat(th, tr("Passengers"), tostring(run.passengers or 0), nil, ctx.can.box),
    } },
  }
  if r.fares then
    out[#out + 1] = P.text(th, tr("Fares %{amount}", { amount = M.money(ctx.assignment, r.fares) }) .. "  •  " .. tr("%{n} tickets", { n = r.tickets or 0 }), "dim", 12.5, "medium")
  end
  return out
end

local function duty_card(ctx)
  local th, tr = ctx.theme, ctx.tr
  local t = totals(1)
  local word = t.score and career.grade(t.score) or nil
  local out = {
    { type = "row", gap = 8, children = {
      { type = "icon", name = "flag", size = 18, color = th.ink },
      { type = "text", text = tr("Duty done"), size = 15, weight = "bold", color = th.ink, grow = true, wrap = false },
      word and grade(ctx, word) or nil,
    } },
    { type = "row", gap = 6, children = {
      P.stat(th, tr("Trips"), tostring(t.trips), nil, ctx.can.box),
      P.stat(th, tr("On time"), t.on_time and (t.on_time .. " %") or "–", nil, ctx.can.box),
      P.stat(th, tr("Score"), t.score and tostring(t.score) or "–", nil, ctx.can.box),
    } },
    { type = "row", gap = 6, children = {
      P.stat(th, tr("Passengers"), tostring(t.passengers), nil, ctx.can.box),
      P.stat(th, tr("km"), string.format("%.1f", t.km), nil, ctx.can.box),
      P.stat(th, tr("Fares"), t.fares and M.money(ctx.assignment, t.fares) or "–", nil, ctx.can.box),
    } },
    { type = "button", id = "ride_summary_close", text = tr("Close"), color = th.neutral },
  }
  return out
end

-- the trip just done while it is fresh
local function fresh(ctx)
  local r = trips[#trips]
  if not r then return nil end
  local age = (ctx.clock or 0) - r.clock
  if age < 0 or age > SHOW_FOR then return nil end
  return r
end

-- the duty's summary applies: its last trip done (still the game's duty) or the duty ended
local function done(ctx)
  if dismissed or #trips == 0 then return false end
  return ctx.phase == "finished" or (ended and not ctx.duty)
end

-- the card that takes the Ride tab over now (nil: none). Not while the walk-around at sign-off
-- is open (the Duty tab's): the summary after it.
function M.mode(ctx)
  if strip.form() == "check" then return nil end
  local r = fresh(ctx)
  if r then return trip_card(ctx, r) end
  if done(ctx) then return duty_card(ctx) end
  return nil
end

-- the slim phone's line for the card shown (nil: none)
function M.slim(ctx)
  if strip.form() == "check" then return nil end
  local th, tr = ctx.theme, ctx.tr
  local r = fresh(ctx)
  local icon, text, value
  if r then
    icon, text, value = "sports_score", tr("Trip done"), tostring((r.judged or {}).score or 0) .. " / 100"
  elseif done(ctx) then
    local t = totals(1)
    icon, text, value = "flag", tr("Duty done"), t.score and (tostring(t.score) .. " / 100") or tostring(t.trips)
  else
    return nil
  end
  local row = { type = "row", gap = 6, children = {
    { type = "icon", name = icon, size = 16, color = th.ink },
    { type = "text", text = text, size = 12.5, weight = "bold", color = th.ink, grow = true, wrap = false },
    { type = "text", text = value, size = 13, weight = "bold", color = th.ink, wrap = false },
  } }
  -- (the duty's summary stays until it is put away: its close button)
  if not r then row.children[4] = { type = "button", id = "ride_summary_close", text = "", icon = "close", color = th.neutral } end
  return row
end

return M
