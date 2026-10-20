-- Omsi-Hub's plugin: the phone's Duty tab (nav.lua) - the company's duty: signing on where
-- Omsi-Hub asks for it, starting the duty and signing off (main.lua's company panel), the duty
-- strip in the phone's look (what is under way, the next break, relief or the duty's end with
-- a countdown bar, today's changes: duty_strip.lua's own reckoning), the IBIS codes to type for
-- the trip and, where the game types them itself, a switch for that; the duty's trips, the
-- one under way marked; the week's roster and the requests the company takes (a sick call, a
-- swap, leave), each asked again before it goes; the strip's note and walk-around.
-- Its modes: a driver still to sign on, and the walk-around at sign-off.
--
-- What it reads of the assignment, each optional: `ibis_codes` as a list of
-- { trip, dep, line, route, dest, text } - the codes to type, found by the trip's departure
-- (seconds of the day), else by the duty's trip from 1; `crew.week` as a list of
-- { day, date, duty, from, to, line, now, swap } - the driver's week (`now`: the duty driven;
-- `swap`: what a swap of it sends back, nil: not to be swapped); `crew.can` as
-- { sick, swap, leave } - the requests Omsi-Hub takes. A request goes back as a reply
-- (`data/replies.jsonl`, bridge::Reply) { kind = "crew", what = "sick", days = 1 },
-- { what = "swap", duty = <the day's swap>, with = 0 } (whoever will) or
-- { what = "leave", from = date, ["until"] = date, reason = "personal" }.

local messages = require("messages")
local signon = require("signon")
local strip = require("duty_strip")
local P = require("phone_theme")

local M = {}

M.id = "duty"
M.title = "Duty"
M.icon = "badge"
-- (the duty's trips listed: as many as there is room for)
M.fit = { 5, 12 }

local WHAT = { "sick", "swap", "leave" }
local LABEL = { sick = "Sick call", swap = "Ask to swap", leave = "Leave request" }
local ICON = { sick = "emergency_home", swap = "sync_alt", leave = "calendar_month" }
local EVENT = { ["break"] = { "Next break", "schedule" }, relief = { "Next relief", "group" }, ["end"] = { "End of duty", "flag" } }

-- what waits to be confirmed: nil, { what = "end" }, or a request ({ what, duty, date, day };
-- no duty yet: its day still to choose)
local pending = nil
-- the requests sent this drive, by what and day
local asked = {}

-- the duty asked to end (the pause menu's "End the duty"): confirmed on the tab
function M.ask_end()
  pending = { what = "end" }
end

-- --- the IBIS ---------------------------------------------------------------------------------

-- the codes to type for trip `k` (`trip` its entry of duty.trips()): { line, route, dest,
-- text } (nil: Omsi-Hub gave none). By the departure first: Omsi-Hub counts only the trips
-- with passengers, and a duty in parts begins its count again.
local function codes(a, k, trip)
  if not (a and k) then return nil end
  local list = type(a.ibis_codes) == "table" and a.ibis_codes or nil
  if not list then return nil end
  local dep = trip and tonumber(trip.departure)
  -- (a departure of 0: not known)
  local function at(c) return tonumber(c.dep) and c.dep > 0 and c.dep or nil end
  for _, c in ipairs(list) do
    if type(c) == "table" and dep and at(c) and math.abs(at(c) - dep) <= 60 then return c end
  end
  for _, c in ipairs(list) do
    if type(c) == "table" and tonumber(c.trip) == k and not (dep and at(c)) then return c end
  end
  return nil
end

local function ibis(ctx, out)
  local th, tr = ctx.theme, ctx.tr
  local c = codes(ctx.assignment, ctx.k, ctx.trip)
  local auto = ctx.has("duty.set_auto_ibis") and ctx.hooks.ibis_auto ~= nil
  if not c and not auto then return end
  out[#out + 1] = { type = "divider" }
  out[#out + 1] = P.label(th, tr("IBIS"))
  if c then
    out[#out + 1] = { type = "row", gap = 6, children = {
      P.stat(th, tr("Line"), tostring(c.line or "–"), nil, ctx.can.box),
      P.stat(th, tr("Route"), tostring(c.route or "–"), nil, ctx.can.box),
      P.stat(th, tr("Destination"), tostring(c.dest or "–"), nil, ctx.can.box),
    } }
    if (c.text or "") ~= "" then out[#out + 1] = P.text(th, c.text, "dim", 12) end
  end
  if auto then
    out[#out + 1] = { type = "checkbox", id = "duty_ibis", text = tr("Type the IBIS for me"), checked = ctx.hooks.ibis_auto() == true, color = th.secondary }
  end
end

-- --- the strip ----------------------------------------------------------------------------------

-- minutes as "in 25 min" / "in 1 h 05 min"
local function until_text(tr, m)
  local left = math.max(math.ceil(m), 0)
  if left >= 60 then return tr("in %{h} h %{m} min", { h = left // 60, m = string.format("%02d", left % 60) }) end
  return tr("in %{m} min", { m = left })
end

-- (whether the duty has a strip)
local function strip_block(ctx, out)
  local th, tr = ctx.theme, ctx.tr
  local now = (ctx.clock or 0) / 60
  local plan = strip.plan(now)
  if not plan then return false end
  out[#out + 1] = { type = "divider" }
  out[#out + 1] = P.text(th, plan.piece, "ink", 13, "medium")
  local first = plan.next[1]
  if first then
    local label, icon = EVENT[first.kind][1], EVENT[first.kind][2]
    local at = first.now and first.to or first.from
    local where = (first.at or "") ~= "" and (first.at .. " · ") or ""
    out[#out + 1] = { type = "row", gap = 8, children = {
      { type = "icon", name = icon, size = 18, color = th.ink },
      { type = "text", text = P.caps(tr(label)), size = 10.5, weight = "medium", color = th.dim, grow = true, wrap = false },
      { type = "text", text = until_text(tr, at - now), size = 15, weight = "bold", color = th.ink, wrap = false },
    } }
    out[#out + 1] = P.text(th, where .. (first.now and tr("now, until %{time}", { time = P.hhmm(first.to * 60) }) or P.hhmm(at * 60)), "dim", 12)
    local span = at - plan.since
    local value = span > 0 and math.min(math.max((now - plan.since) / span, 0), 1) or 1
    out[#out + 1] = { type = "bar", value = value, height = 4, color = th.ink, background = th.track }
  end
  for i = 2, #plan.next do
    local e = plan.next[i]
    out[#out + 1] = { type = "row", gap = 8, children = {
      { type = "icon", name = EVENT[e.kind][2], size = 14, color = th.dim },
      { type = "text", text = tr(EVENT[e.kind][1]), size = 12, color = th.dim, grow = true, wrap = false },
      { type = "text", text = P.hhmm(e.from * 60) .. ((e.at or "") ~= "" and ("  " .. e.at) or ""), size = 12, color = th.ink, wrap = false },
    } }
  end
  for _, ch in ipairs(plan.changes) do
    out[#out + 1] = P.text(th, "• " .. tostring(ch), "ink", 12)
  end
  return true
end

-- --- the trips -----------------------------------------------------------------------------------

-- the duty's trips: done, under way, to come (from the one before the trip under way, as many
-- as there is room for)
local function trip_list(ctx, out)
  local th, tr, d = ctx.theme, ctx.tr, ctx.duty
  out[#out + 1] = { type = "divider" }
  if #ctx.trips == 0 then
    out[#out + 1] = P.text(th, tr("No trips yet: start your duty"), "dim")
    return
  end
  local most = math.max(ctx.fit or M.fit[1], 2)
  local first = math.max(1, math.min((d and d.trip or 1) - 1, #ctx.trips - most + 1))
  local rows = {}
  for k = first, math.min(#ctx.trips, first + most - 1) do
    local t = ctx.trips[k]
    local done = d and (k < d.trip or (k == d.trip and d.trip_done))
    local now = d and k == d.trip and not d.trip_done
    local ink = done and th.faint or (now and th.ink or th.dim)
    rows[#rows + 1] = {
      { text = done and "✓" or (now and "▶" or tostring(k)), color = ink },
      { text = t.line or "", color = ink },
      { text = P.hhmm(t.departure), color = ink },
      { text = P.terminus(t.line, t.terminus), color = ink },
      { text = P.hhmm(t.arrival), color = ink },
    }
  end
  local more = #ctx.trips - (first + #rows - 1)
  if more > 0 then rows[#rows + 1] = { { text = "", color = th.faint }, { text = "", color = th.faint }, { text = "+" .. more, color = th.faint }, { text = "", color = th.faint }, { text = "", color = th.faint } } end
  if not ctx.can.box then
    -- (an openOMSI before coloured cells writes a cell's table as nothing: plain texts there)
    for _, r in ipairs(rows) do
      for i, cell in ipairs(r) do r[i] = cell.text end
    end
  end
  out[#out + 1] = { type = "table", columns = { "", tr("Line"), tr("Dep."), "", tr("Arr.") }, widths = { 0.08, 0.14, 0.16, 0.46, 0.16 },
    aligns = { "center", "left", "left", "left", "right" }, rows = rows, size = 12.5, color = th.ink }
end

-- --- the week and the requests -----------------------------------------------------------------

local function crew(ctx)
  local a = ctx.assignment
  return a and type(a.crew) == "table" and a.crew or {}
end

local function week(ctx)
  local w = crew(ctx).week
  return type(w) == "table" and w or {}
end

-- a day of the week as its button says it: "Mon 20"
local function day_label(r)
  local dd = tostring(r.date or ""):match("%-(%d%d)$")
  return tostring(r.day or "") .. (dd and (" " .. tonumber(dd)) or "")
end

-- today's duty and date: the roster's duty driven now, else the week's first day (the duty's)
local function today(ctx)
  local duty = ctx.assignment and ctx.assignment.duty or ""
  local w = week(ctx)
  for _, r in ipairs(w) do
    if r.now == true then return r.duty or duty, r.date or "", day_label(r) end
  end
  if w[1] then return duty, w[1].date or "", day_label(w[1]) end
  return duty, "", ""
end

-- the days a request can be for: a swap those Omsi-Hub says can be swapped, leave those with
-- a duty after today (the company takes no leave for today)
local function duty_days(ctx, what)
  local out = {}
  local _, now = today(ctx)
  for i, r in ipairs(week(ctx)) do
    if what == "swap" then
      if type(r.swap) == "table" then out[#out + 1] = i end
    elseif (r.duty or "") ~= "" and tostring(r.date or "") > now then
      out[#out + 1] = i
    end
  end
  return out
end

local function roster(ctx, out)
  local th, tr = ctx.theme, ctx.tr
  local w = week(ctx)
  if #w == 0 then return end
  out[#out + 1] = { type = "divider" }
  out[#out + 1] = P.label(th, tr("This week"))
  local rows = {}
  for _, r in ipairs(w) do
    local ink = r.now == true and th.ink or th.dim
    local off = (r.duty or "") == ""
    rows[#rows + 1] = {
      { text = day_label(r), color = ink },
      { text = off and tr("Day off") or tostring(r.duty), color = ink },
      { text = off and "" or (tostring(r.from or "") .. "–" .. tostring(r.to or "")), color = ink },
      { text = tostring(r.line or ""), color = ink },
    }
  end
  if not ctx.can.box then
    for _, r in ipairs(rows) do
      for i, cell in ipairs(r) do r[i] = cell.text end
    end
  end
  out[#out + 1] = { type = "table", columns = { tr("Day"), tr("Duty"), tr("Time"), tr("Line") }, widths = { 0.24, 0.24, 0.34, 0.18 }, rows = rows, size = 12, color = th.ink }
end

-- the question a request or the duty's end waits on, and its two buttons
local function confirm(ctx, out)
  local th, tr = ctx.theme, ctx.tr
  local p = pending
  if p.what == "end" then
    out[#out + 1] = P.text(th, tr("End the duty now? The trip under way counts as not completed."), "ink", 13, "bold")
  elseif not p.duty then
    -- (a swap or leave: which day first)
    local ask = p.what == "swap" and tr("Swap which day?") or tr("Leave on which day?")
    out[#out + 1] = P.text(th, ask, "ink", 13, "bold")
    local row = nil
    for n, i in ipairs(duty_days(ctx, p.what)) do
      if (n - 1) % 4 == 0 then
        row = { type = "row", gap = 6, children = {} }
        out[#out + 1] = row
      end
      row.children[#row.children + 1] = { type = "button", id = "duty_day_" .. i, text = day_label(week(ctx)[i]), color = th.neutral, grow = true }
    end
    out[#out + 1] = { type = "button", id = "duty_no", text = tr("Cancel"), color = th.neutral }
    return
  elseif p.what == "sick" then
    out[#out + 1] = P.text(th, tr("Call in sick for duty %{duty}? The company finds cover.", { duty = p.duty }), "ink", 13, "bold")
  elseif p.what == "swap" then
    out[#out + 1] = P.text(th, tr("Ask to swap duty %{duty} on %{day}?", { duty = p.duty, day = p.day }), "ink", 13, "bold")
  else
    out[#out + 1] = P.text(th, tr("Ask for leave on %{day} (duty %{duty})?", { duty = p.duty, day = p.day }), "ink", 13, "bold")
  end
  local yes = p.what == "end" and tr("End the duty") or tr("Send")
  out[#out + 1] = { type = "row", gap = 6, children = {
    { type = "button", id = "duty_yes", text = yes, icon = p.what == "end" and "flag" or "check", color = th.primary, grow = true },
    { type = "button", id = "duty_no", text = tr("Cancel"), icon = "close", color = th.neutral, grow = true },
  } }
end

-- the requests the company takes, as buttons; those sent this drive said
local function requests(ctx, out)
  local th, tr = ctx.theme, ctx.tr
  local can = crew(ctx).can
  if type(can) ~= "table" then return end
  local row = { type = "row", gap = 6, children = {} }
  for _, w in ipairs(WHAT) do
    -- (a swap or leave only with a day it can be for)
    if can[w] == true and (w == "sick" or #duty_days(ctx, w) > 0) then
      row.children[#row.children + 1] = { type = "button", id = "duty_" .. w, text = tr(LABEL[w]), icon = ICON[w], color = th.neutral, grow = true }
    end
  end
  if #row.children == 0 then return end
  if #week(ctx) == 0 then out[#out + 1] = { type = "divider" } end
  out[#out + 1] = row
  for _, text in ipairs(asked) do out[#out + 1] = P.text(th, "✓ " .. text, "dim", 12) end
end

-- --- the tab --------------------------------------------------------------------------------------

-- the duty, starting it and signing off, in the phone's look
local function head(ctx, out)
  local th, tr, a = ctx.theme, ctx.tr, ctx.assignment
  if not (a and (a.duty or "") ~= "") then return end
  out[#out + 1] = { type = "row", gap = 8, children = {
    { type = "icon", name = "badge", size = 18, color = th.dim },
    { type = "text", text = tr("Duty %{duty}: line %{line}, tour %{tour}", { duty = a.duty, line = a.line or "", tour = a.tour or "" }), size = 13, weight = "bold", color = th.ink, grow = true },
  } }
  local can_start = not ctx.duty and ctx.has("duty.start") and (a.line or "") ~= ""
  local buttons = {}
  if can_start then buttons[#buttons + 1] = { type = "button", id = "start", text = tr("Start duty"), icon = "play_arrow", color = th.primary, grow = true } end
  if signon.signed() then buttons[#buttons + 1] = { type = "button", id = "signoff", text = tr("Sign off"), icon = "logout", color = th.neutral, grow = true } end
  if #buttons > 0 then out[#out + 1] = { type = "row", gap = 6, children = buttons } end
end

function M.rows(ctx)
  local out = {}
  head(ctx, out)
  -- (the duty ended meanwhile: nothing left to end)
  if pending and pending.what == "end" and not ctx.duty then pending = nil end
  if pending then
    out[#out + 1] = { type = "divider" }
    confirm(ctx, out)
  end
  if strip.form() == "note" then strip.tools(out) end
  local strip_shown = ctx.at_work and strip_block(ctx, out)
  if ctx.duty then
    ibis(ctx, out)
    trip_list(ctx, out)
  end
  roster(ctx, out)
  requests(ctx, out)
  if strip_shown and strip.form() ~= "note" then strip.tools(out) end
  if #out == 0 then out[1] = P.text(ctx.theme, ctx.tr("No duty: take one in the Omsi-Hub app"), "dim") end
  return out
end

-- signing on, and the walk-around at sign-off: the tab is about nothing else then
function M.mode(ctx)
  if ctx.hooks.duty_children and not signon.at_work() then return ctx.hooks.duty_children() end
  if strip.form() == "check" then return strip.tools() end
  return nil
end

function M.slim(ctx)
  if ctx.hooks.duty_children and not signon.at_work() then return P.text(ctx.theme, ctx.tr("Not signed on"), "ink", 12.5, "bold") end
  return nil
end

-- a request sent: as a reply to Omsi-Hub (bridge::Reply: `duty` only ever the week's swap
-- table, or the whole line is refused), said on a card
local function send(ctx, p)
  local extra = { what = p.what }
  if p.what == "sick" then
    extra.days = 1
  elseif p.what == "swap" then
    extra.duty, extra.with = p.swap, 0
  else
    extra.from, extra["until"], extra.reason = p.date, p.date, "personal"
  end
  messages.reply("crew", 0, p.what, extra)
  local text = ctx.tr(LABEL[p.what]) .. ((p.day or "") ~= "" and (" · " .. p.day) or "")
  asked[#asked + 1] = text
  if ctx.hooks.say then ctx.hooks.say(ctx.tr("Sent to the company"), text, ICON[p.what], 6) end
end

function M.click(ctx, id)
  if id:sub(1, 5) ~= "duty_" then return false end
  local w = id:sub(6)
  if LABEL[w] then
    if w == "sick" then
      local duty, date, day = today(ctx)
      pending = { what = w, duty = duty, date = date, day = day }
    else
      pending = { what = w }
    end
  elseif w:match("^day_%d+$") and pending and pending.what ~= "end" then
    local r = week(ctx)[tonumber(w:sub(5))]
    if r then pending = { what = pending.what, duty = r.duty or "", date = r.date or "", day = day_label(r), swap = r.swap } end
  elseif w == "yes" and pending then
    local p = pending
    pending = nil
    if p.what == "end" then
      if ctx.hooks.end_duty then ctx.hooks.end_duty() end
    elseif p.duty then
      send(ctx, p)
    end
  elseif w == "no" then
    pending = nil
  else
    return false
  end
  return true
end

function M.change(ctx, id, value)
  if id ~= "duty_ibis" then return false end
  if ctx.hooks.set_ibis_auto then ctx.hooks.set_ibis_auto(value == true) end
  return true
end

return M
