-- Omsi-Hub's plugin: the phone's Ride tab (nav.lua) - the drive now: the stop the bus heads for,
-- large on a card of Omsi-Hub's blue, with when it gets there, the one after it and how far
-- along the trip it is; the
-- passengers, the speed and the delay; the lines that apply now - the gap to the bus ahead and
-- how long to hold, a connection to wait for, a diversion, the control room's instruction
-- waiting in Messages, the punctuality coach - and the trip's progress.
-- Its modes: the trip's card as a trip ends and the duty's summary as it ends
-- (phone_summary.lua), and at a terminus with time to spare the break clock counting down to
-- the next departure.

local company = require("company")
local ontime = require("ontime")
local stops = require("phone_stops")
local summary = require("phone_summary")
local P = require("phone_theme")

local M = {}

M.id = "ride"
M.title = "Ride"
M.icon = "navigation"
-- (in the navigator's whole frame: its map over the drive)
M.map = true
-- (the lines that apply now: as many as the room has room for)
M.fit = { 3, 4 }

-- (the control room's messages are read again at most this often for the instruction, s)
local READ_EVERY = 5

-- a text from elsewhere cut to `most` characters (the game refuses a panel with a longer one)
local function cut(s, most)
  s = tostring(s or "")
  local n = utf8.len(s)
  if not n then return s:sub(1, most) end
  if n <= most then return s end
  return s:sub(1, utf8.offset(s, most) - 1) .. "…"
end

-- passengers, km/h and the delay, side by side (the delay in the time's colour)
local function stats(ctx)
  local th, tr = ctx.theme, ctx.tr
  local text, color = P.delay(ctx.delay, th, tr)
  return { type = "row", gap = 6, children = {
    P.stat(th, tr("Passengers"), tostring(math.tointeger(ctx.passengers or 0) or 0), nil, ctx.can.box),
    P.stat(th, tr("km/h"), tostring(math.floor((ctx.speed or 0) + 0.5)), nil, ctx.can.box),
    P.stat(th, tr("Delay"), text, color, ctx.can.box),
  } }
end

-- the stop the bus heads for, large: its name, stop 2/7; when the bus is there (a pill, in the
-- time's colour off the timetable) and in how many minutes; the one after it
local function next_stop(ctx, served, now)
  local th, tr, d = ctx.theme, ctx.tr, ctx.duty
  local s = served[now]
  local time, color, left
  if ctx.phase == "before" then
    local leaves = tonumber(d.departure) or (ctx.trip or {}).departure or s.departure or ctx.clock
    time, color, left = tr("leaves %{time}", { time = P.hhmm(leaves) }), th.dim, leaves - ctx.clock
  else
    local expected, ink = stops.expected(ctx, s)
    time = tr("arr %{time}", { time = expected or P.hhmm(s.arrival) })
    color = ink
    left = (s.arrival or ctx.clock) + (ctx.delay or 0) - ctx.clock
  end
  local m = math.ceil(left / 60)
  local after = served[now + 1]
  -- (off the timetable a pill of the time's colour with white text, as the status line's)
  local timed = color == th.late or color == th.early
  return {
    { type = "row", gap = 6, children = {
      { type = "icon", name = "near_me", size = 14, color = th.accent },
      { type = "text", text = P.caps(tr("Next stop")), size = 10.5, weight = "bold", color = th.accent, grow = true, wrap = false },
      { type = "text", text = tr("stop %{i}/%{n}", { i = now, n = #served }), size = 11.5, weight = "medium", color = th.dim, wrap = false },
    } },
    { type = "text", text = s.name or "", size = 20, weight = "bold", color = th.ink, wrap = false },
    { type = "row", gap = 8, children = {
      { type = "badge", text = time, color = timed and color or th.chip, text_color = timed and "#FFFFFF" or th.ink, weight = "bold" },
      { type = "text", text = m > 0 and tr("in %{m} min", { m = m }) or tr("due now"), size = 13, weight = "medium", color = th.ink, grow = true, wrap = false },
    } },
    { type = "text", size = 12, color = th.dim, wrap = false,
      text = after and tr("then %{stop}", { stop = after.name or "" }) or tr("terminus") },
  }
end

-- the trip's progress: the stops behind the bus and those ahead
local function progress(ctx, served, now)
  local th = ctx.theme
  local done = (now or #served + 1) - 1
  local bar = { type = "bar", value = done / math.max(#served, 1), height = 4, color = th.accent, background = th.track }
  if ctx.can.segments then
    bar.segments = { { value = done, color = th.accent }, { value = #served - done, color = th.track } }
  end
  return bar
end

-- the control room's instruction waiting for the driver's answer: its text (read again from
-- the inbox's file now and then; nil: none, or no file to read)
local said = { left = 0, text = nil }
local function instruction(ctx)
  said.left = said.left - 1
  if said.left > 0 then return said.text end
  said.left, said.text = READ_EVERY, nil
  if not (ctx.has("files.read") and ctx.has("json.decode")) then return nil end
  local ok, inbox = pcall(omsi.json.decode, omsi.files.read("messages.json") or "")
  for _, m in ipairs(ok and type(inbox) == "table" and type(inbox.messages) == "table" and inbox.messages or {}) do
    if type(m) == "table" and m.action and not m.mine and m.state ~= "accepted" and m.state ~= "declined" then
      said.text = cut(m.text, 200)
      break
    end
  end
  return said.text
end

-- a line that applies now: its icon (in the accent: it points the way) and text (`one`: on
-- one line, cut where it is longer)
local function line(ctx, icon, text, extra, one)
  local th = ctx.theme
  local children = {
    { type = "icon", name = icon, size = 16, color = th.accent },
    { type = "text", text = text, size = 12.5, color = th.ink, grow = true, wrap = not one },
  }
  if extra then children[3] = extra end
  return { type = "row", gap = 8, children = children }
end

-- the minutes of a gap in seconds
local function minutes(s)
  return math.floor((tonumber(s) or 0) / 60 + 0.5)
end

-- the lines that apply now, the most pressing first: the instruction waiting, the headway,
-- a connection, a diversion, the coach (Omsi-Hub's brief: ontime.lua reads it)
local function context(ctx)
  local th, tr = ctx.theme, ctx.tr
  local out = {}
  if ctx.wants then
    local text = instruction(ctx) or tr("Message from control")
    -- (a pointer to Messages, on one line: the message itself is read there)
    out[#out + 1] = line(ctx, "forum", text, { type = "button", id = "ride_inbox", text = tr("Open"), color = th.primary }, true)
  end
  local b = ontime.brief() or {}
  local h = b.headway
  if type(h) == "table" then
    local parts = {}
    if type(h.ahead) == "table" then
      parts[#parts + 1] = tr("%{bus} ahead %{m} min", { bus = h.ahead.bus or "", m = minutes(h.ahead.secs) })
    elseif type(h.behind) == "table" then
      parts[#parts + 1] = tr("%{bus} behind %{m} min", { bus = h.behind.bus or "", m = minutes(h.behind.secs) })
    end
    local a = type(h.advice) == "table" and h.advice or {}
    if a.kind == "hold" then
      parts[#parts + 1] = tr("hold %{secs} s", { secs = math.floor((tonumber(a.secs) or 0) + 0.5) })
    elseif a.kind == "close_behind" then
      parts[#parts + 1] = tr("close behind %{bus}", { bus = a.bus or "" })
    end
    if #parts > 0 then out[#out + 1] = line(ctx, a.kind == "hold" and "timer" or "directions_bus", table.concat(parts, "  •  ")) end
  end
  local k = type(b.connections) == "table" and b.connections[1] or nil
  if type(k) == "table" then
    local due = (k.time or "") .. ((tonumber(k.late) or 0) > 0 and (" (+" .. k.late .. " min)") or "")
    local text = tr("At %{stop}: %{what} %{time}", { stop = cut(k.stop, 60), what = cut(k.what, 100), time = cut(due, 30) })
    local st = type(k.status) == "table" and k.status or {}
    -- (one the bus waits for: what Omsi-Hub says of it now, as the on-time card did; the
    -- company's longest wait where the brief says nothing)
    if not k.onward and st.kind == "wait" then
      text = text .. "  •  " .. tr("Hold until %{time}", { time = cut(st["until"], 10) })
    elseif not k.onward and st.kind == "too_late" then
      text = text .. "  •  " .. tr("Too late to wait for: leave on time")
    elseif not k.onward and st.kind == "not_needed" then
      text = text .. "  •  " .. tr("No wait needed")
    elseif not k.onward then
      text = text .. "  •  " .. tr("wait max %{m} min", { m = k.max_hold or 0 })
    elseif st.kind == "made" then
      text = text .. "  •  " .. tr("%{m} min to change", { m = st.m or 0 })
    elseif st.kind == "tight" then
      text = text .. "  •  " .. tr("Tight: keep the doors short")
    else
      text = text .. "  •  " .. tr("Missed at this delay")
    end
    out[#out + 1] = line(ctx, "commute", text)
  end
  if type(b.ahead) == "table" then
    local closed = {}
    for _, s in ipairs(type(b.closed) == "table" and b.closed or {}) do closed[#closed + 1] = tostring(s) end
    if #closed == 0 then closed[1] = b.ahead.stop or "" end
    closed = cut(table.concat(closed, ", "), 200)
    local via = cut(b.ahead.via or b.via or "", 100)
    local text = via ~= "" and tr("Diversion: %{stops} closed, via %{via}", { stops = closed, via = via })
      or tr("Diversion: %{stops} closed", { stops = closed })
    out[#out + 1] = line(ctx, "alt_route", text)
  end
  local parts = ctx.hooks.parts and ctx.hooks.parts() or {}
  if parts.coach and ctx.duty then
    local contract = type(ctx.assignment) == "table" and type(ctx.assignment.contract) == "table" and ctx.assignment.contract or nil
    local c = company.coach(ctx.duty, ctx.trips, ctx.clock, contract)
    if c then out[#out + 1] = line(ctx, c.icon or "schedule", c.text) end
  end
  -- (as many as fit, the first ones)
  for j = #out, math.max(ctx.fit or M.fit[1], 1) + 1, -1 do out[j] = nil end
  return out
end

function M.rows(ctx)
  local th, tr = ctx.theme, ctx.tr
  if not ctx.duty then
    if summary.ended() then return { P.text(th, tr("Duty finished"), "dim") } end
    return { P.text(th, ctx.at_work and tr("Not started yet") or tr("Not signed on"), "dim") }
  end
  if ctx.phase == "finished" then return { P.text(th, tr("Duty finished"), "dim") } end
  local out = {}
  local served, now = stops.served(ctx)
  if now then
    -- (the stop ahead and the trip's progress on one card)
    local card = next_stop(ctx, served, now)
    if #served > 1 then
      card[#card + 1] = { type = "space", size = 2 }
      card[#card + 1] = progress(ctx, served, now)
    end
    P.card(out, th, ctx.can.box, card, th.primary, 3)
  elseif #served > 0 then
    out[#out + 1] = P.text(th, tr("At the terminus: %{stop}", { stop = served[#served].name or "" }), "ink", 15, "bold")
    if #served > 1 then out[#out + 1] = progress(ctx, served, now) end
  end
  out[#out + 1] = stats(ctx)
  for _, e in ipairs(context(ctx)) do out[#out + 1] = e end
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
  local out = {}
  -- (the clock on a card: Omsi-Hub's blue, the time's red once the break is over)
  P.card(out, th, ctx.can.box, {
    { type = "row", gap = 8, children = {
      { type = "icon", name = "schedule", size = 16, color = over and th.late or th.accent },
      { type = "text", text = over and tr("Break over") or tr("Break"), size = 13, weight = "bold", color = over and th.late or th.ink, grow = true },
      { type = "text", text = tr("until %{time}", { time = P.hhmm(after.departure) }), size = 12, color = th.dim, wrap = false },
    } },
    { type = "text", text = (over and "+" or "") .. P.mmss(math.abs(left)), size = 40, weight = "bold", align = "center", color = ink },
    { type = "bar", value = math.min(math.max(1 - left / length, 0), 1), height = 4, color = over and th.late or th.accent, background = th.track },
  }, over and th.late or th.primary, 4)
  local more = {
    { type = "row", gap = 7, children = {
      { type = "text", text = tr("Next %{time}", { time = P.hhmm(after.departure) }), size = 12.5, weight = "bold", color = th.ink },
      P.plate(th, after.line or "", tr),
      { type = "text", text = P.terminus(after.line, after.terminus), size = 12.5, color = th.ink, grow = true, wrap = false },
    } },
  }
  for _, e in ipairs(more) do out[#out + 1] = e end
  if (after.line or "") ~= (this.line or "") and (after.line or "") ~= "" then
    out[#out + 1] = P.text(th, tr("Continue on line %{line}, tour %{tour}", { line = after.line, tour = d.tour or "" }), "ink", 12, "bold")
  end
  return out
end

function M.mode(ctx)
  local m = summary.mode(ctx)
  if m then return m end
  if ctx.duty and ctx.phase == "break" then return break_clock(ctx) end
  return nil
end

-- the slim phone: the card shown, the break's minutes, else the next stop and in how long
function M.slim(ctx)
  local th, tr = ctx.theme, ctx.tr
  local card = summary.slim(ctx)
  if card then return card end
  if not ctx.duty then return nil end
  if ctx.phase == "break" then
    local after = ctx.trips[ctx.duty.trip + 1] or {}
    return { type = "row", gap = 6, children = {
      { type = "icon", name = "schedule", size = 16, color = th.dim },
      { type = "text", text = tr("Break"), size = 12.5, weight = "bold", color = th.ink, grow = true },
      { type = "text", text = P.mmss((after.departure or ctx.clock) - ctx.clock), size = 14, weight = "bold", color = th.ink },
    } }
  end
  local served, now = stops.served(ctx)
  local s = now and served[now]
  if not s then return nil end
  local expected, ink = stops.expected(ctx, s)
  return { type = "row", gap = 6, children = {
    { type = "icon", name = "near_me", size = 16, color = th.accent },
    { type = "text", text = s.name or "", size = 12.5, weight = "bold", color = th.ink, grow = true, wrap = false },
    { type = "text", text = expected or P.hhmm(s.arrival), size = 12.5, weight = "medium", color = ink or th.dim },
  } }
end

-- the instruction's button opens Messages; the duty's summary put away
function M.click(ctx, id)
  if id == "ride_inbox" then
    ctx.show("messages")
    return true
  end
  if id == "ride_summary_close" then
    summary.dismiss()
    return true
  end
  return false
end

return M
