-- Omsi-Hub's plugin: the duty strip of a company's duty (Luc, 2026-10-16) - the piece of work
-- under way, the next break and the next relief with where, when and how long until then, the
-- end of the duty, what changed today (an extra duty, overtime, a changed relief, a swap), the
-- bus's fuel or charge with "return to the depot" when it runs low; and the driver's side of the
-- bus: the handover notes left on it (shown at sign-on), a note of one's own for the next driver,
-- and the walk-around at sign-off whose defects go to the company's workshop. A collision the
-- game sees is logged to the workshop by itself.
--
-- Self-contained: in the navigator under the duty board (navigator.lua; an older openOMSI: a
-- panel of its own, `omsihub_strip`), its data from the assignment's `crew`
-- (Omsi-Hub's `bridge::Crew`, made from the RosterFlow duty the company launched), and what it
-- reports goes through the `report` main.lua hands it. Switched off by the assignment
-- (`crew.strip = false`) or by another module that draws a strip of its own (`M.set_enabled`):
-- there is only ever one duty strip on the screen.

local layout = require("layout")
local navigator = require("navigator")

local M = {}

local PANEL = "omsihub_strip"
local GREEN, AMBER, RED, DIM, ACCENT = "#37D67A", "#F0B429", "#FF5F57", "#9AA3AD", "#F47F30"
-- (a stand shorter than this is a layover, not a break; the relief is told this long before)
local BREAK_MIN = 10
local RELIEF_WARN = 10
-- (collisions are logged at most this often, s)
local FAULT_EVERY = 120

local tr = function(t) return t end
local crew = nil
local report = function() end
local notify = function() end

local enabled = true
local shown = false
-- what the driver has open: nil, "note" or "check"
local open = nil
local note = { template = nil, text = "" }
local check = { ticked = {}, text = "", done = false }
local told = {}
local fuel = { level = nil, low = false, most = nil, told = false }
local fault = { base = nil, at = -FAULT_EVERY, clock = 0 }
local look = 0
-- whether the driver is at work (signed on, or not needing to)
local at_work = false

-- `translate` the plugin's texts, `c` the assignment's crew (nil: not a company's duty),
-- `send(kind, fields)` a report to Omsi-Hub, `say(icon, title, text)` a notification
function M.setup(translate, c, send, say)
  tr, crew, report, notify = translate, c, send or report, say or notify
  if type(crew) ~= "table" or type(crew.rows) ~= "table" then crew = nil end
  enabled = crew ~= nil and crew.strip ~= false
end

-- whether the strip is shown (another module may take the place: `set_enabled(false)`)
function M.enabled()
  return enabled
end

function M.set_enabled(on)
  enabled = on and crew ~= nil
  if not enabled then navigator.put("strip", nil) end
  if not enabled and shown and omsi.ui and omsi.ui.remove then
    layout.remove(PANEL)
    shown = false
  end
end

local function fn(t, name)
  return type(t) == "table" and type(t[name]) == "function"
end

local function hhmm(m)
  m = math.floor(m or 0)
  return string.format("%02d:%02d", (m // 60) % 24, m % 60)
end

-- the game's clock in minutes of the day
local function minute()
  local i = fn(omsi, "info") and omsi.info() or nil
  local s = type(i) == "table" and tonumber(i.clock) or 0
  return (s or 0) / 60
end

-- "in 5 min" for `left` whole minutes to go
local function until_text(left)
  if left >= 60 then return tr("in %{h} h %{m} min", { h = left // 60, m = left % 60 }) end
  return tr("in %{m} min", { m = left })
end

-- --- what the duty looks like now ---------------------------------------------------------------

-- the piece under way at minute `now`: its text, its kind and its tone
local function piece(now)
  local rows = crew.rows
  local first, last = rows[1], rows[#rows]
  if first and now < first.from then
    return { kind = "sign_on", tone = "dim", text = tr("Sign on at %{place} at %{time}", { place = first.at, time = hhmm(first.from) }) }
  end
  for _, r in ipairs(rows) do
    if now >= r.from and now < math.max(r.to, r.from + 1) then
      if r.kind == "trip" and r.empty then return { kind = "empty", tone = "dim", text = tr("Empty run to %{place} until %{time}", { place = r.to_place, time = hhmm(r.to) }) } end
      if r.kind == "trip" then return { kind = "trip", tone = "good", text = tr("Line %{line} to %{place} until %{time}", { line = r.line, place = r.to_place, time = hhmm(r.to) }) } end
      if r.kind == "break" then return { kind = "break", tone = "warn", text = tr("Break at %{place} until %{time}", { place = r.at, time = hhmm(r.to) }) } end
    end
  end
  if last and now >= last.from then return { kind = "finished", tone = "good", text = tr("Duty finished") } end
  return { kind = "duty", tone = "good", text = tr("On duty") }
end

-- the next break (or the one under way) after minute `now`
local function next_break(now)
  for _, r in ipairs(crew.rows) do
    if r.kind == "break" and r.to - r.from >= BREAK_MIN and r.to > now then return r end
  end
  return nil
end

-- the next relief: where the driver takes a bus over or hands it on
local function next_relief(now)
  for _, r in ipairs(crew.rows) do
    if (r.kind == "sign_on" or r.kind == "sign_off") and r.relief and r.from >= now then return r end
  end
  return nil
end

-- --- the bus: fuel and faults --------------------------------------------------------------------

-- the fuel (or the charge) left, 0 to 1, where the bus tells it
local function fuel_level()
  if not omsi.bus then return nil end
  -- an electric bus's charge, where its script has one (0..1 or 0..100)
  if fn(omsi.bus, "get_vars") then
    local v = omsi.bus.get_vars({ "elec_battery_soc", "battery_soc", "soc", "engine_tank_capacity" })
    if type(v) == "table" then
      for _, k in ipairs({ "elec_battery_soc", "battery_soc", "soc" }) do
        local x = tonumber(v[k])
        if x then return x > 1 and math.min(x / 100, 1) or math.max(x, 0), "charge" end
      end
      local cap = tonumber(v.engine_tank_capacity)
      if cap and cap > 0 then fuel.most = cap end
    end
  end
  local litres = fn(omsi.bus, "fuel") and tonumber(omsi.bus.fuel()) or nil
  if not litres then return nil end
  -- (without the tank's size: the most seen since the drive began, the tank as it came)
  fuel.most = math.max(fuel.most or 0, litres)
  if fuel.most <= 0 then return nil end
  return math.min(litres / fuel.most, 1), "fuel"
end

-- what the bus says of itself, looked at once a second: the fuel, a collision
function M.frame(dt)
  if not crew then return end
  look = look - (dt or 0)
  fault.clock = fault.clock + (dt or 0)
  if look > 0 then return end
  look = 1
  local level, kind = fuel_level()
  fuel.level, fuel.kind = level, kind
  local low = level ~= nil and level < (tonumber(crew.low_fuel) or 0.15)
  if low and not fuel.told then
    fuel.told = true
    notify("local_gas_station", tr("Fuel low"), tr("Return to the depot to refuel"))
  elseif not low and level and level > (tonumber(crew.low_fuel) or 0.15) + 0.1 then
    fuel.told = false
  end
  fuel.low = low
  -- a collision: logged to the workshop by itself (once in a while, not every bump)
  if fn(omsi.bus, "damage") then
    local d = omsi.bus.damage()
    local n = type(d) == "table" and tonumber(d.crashes) or nil
    if n then
      if fault.base == nil then fault.base = n end
      if n > fault.base and fault.clock - fault.at >= FAULT_EVERY then
        fault.base, fault.at = n, fault.clock
        report("fault", { defect = "other", text = "", minutes = tonumber(d.repair_minutes), bus_id = crew.bus })
      elseif n > fault.base then
        fault.base = n
      end
    end
  end
end

-- --- the strip ----------------------------------------------------------------------------------

local function line(icon, label, value, color)
  return { type = "row", children = {
    { type = "icon", name = icon, color = color or DIM, size = 16 },
    { type = "text", text = label, size = 12, color = DIM, wrap = false },
    { type = "text", text = value, size = 13, color = color, grow = true, align = "right" },
  } }
end

local function note_form(children)
  local function add(x) children[#children + 1] = x end
  add({ type = "divider" })
  add({ type = "text", text = tr("A note for the next driver of bus %{bus}", { bus = crew.bus_number or "" }), weight = "bold", size = 13 })
  for _, t in ipairs(crew.templates or {}) do
    add({ type = "checkbox", id = "strip_tpl_" .. t[1], text = t[2], checked = note.template == t[1] })
  end
  add({ type = "input", id = "strip_note_text", text = note.text, placeholder = tr("Or type a note"), max = tonumber(crew.text_max) or 140 })
  add({ type = "row", children = {
    { type = "button", id = "strip_note_send", text = tr("Leave the note"), icon = "sticky_note_2", grow = true },
    { type = "button", id = "strip_close", text = tr("Close"), grow = true },
  } })
end

local function check_form(children)
  local function add(x) children[#children + 1] = x end
  add({ type = "divider" })
  add({ type = "text", text = tr("Walk-around check of bus %{bus}", { bus = crew.bus_number or "" }), weight = "bold", size = 13 })
  add({ type = "text", text = tr("Tick what is not in order: the workshop gets it."), size = 12, color = DIM })
  for _, k in ipairs(crew.checks or {}) do
    add({ type = "checkbox", id = "strip_chk_" .. k[1], text = k[2], checked = check.ticked[k[1]] == true })
  end
  add({ type = "input", id = "strip_chk_text", text = check.text, placeholder = tr("What is wrong (optional)"), max = tonumber(crew.text_max) or 140 })
  add({ type = "row", children = {
    { type = "button", id = "strip_chk_send", text = tr("Report defects"), icon = "build", grow = true },
    { type = "button", id = "strip_chk_ok", text = tr("All in order"), icon = "task_alt", grow = true },
  } })
end

-- --- the strip as data ----------------------------------------------------------------------------

local TONE = { dim = DIM, good = GREEN, warn = AMBER }

-- (the minutes to go until minute `m`, whole, none under 0)
local function minutes_to(m, now)
  return math.max(math.ceil(m - now), 0)
end

-- the strip's facts at minute `now`, plain data (the strip's elements and the on-board computer
-- both read them; a list with nothing in it is left out): nil when the strip is off (no
-- company's duty, or another strip has the place)
local function facts_at(now)
  if not (crew and enabled) then return nil end
  local out = {
    signed_on = at_work, piece = piece(now), relief_warning = false,
    can_note = true, can_check = true, text_max = tonumber(crew.text_max) or 140,
  }
  local b = next_break(now)
  if b then
    out.next_break = { at = b.at or "", from = hhmm(b.from), to = hhmm(b.to), under_way = b.from <= now, left = minutes_to(b.from, now) }
  end
  local r = next_relief(now)
  if r then
    local soon = r.from - now <= RELIEF_WARN
    out.next_relief = { kind = r.kind, at = r.at or "", from = hhmm(r.from), left = minutes_to(r.from, now), soon = soon }
    out.relief_warning = soon
  end
  local last = crew.rows[#crew.rows]
  if last then out.ends = { at = last.at or "", from = hhmm(last.from) } end
  if #(crew.changes or {}) > 0 then out.changes = { table.unpack(crew.changes) } end
  if fuel.level then
    out.fuel = { level = math.floor(fuel.level * 1000 + 0.5) / 1000, kind = fuel.kind, low = fuel.low == true, to_depot = fuel.low == true }
  end
  if #(crew.notes or {}) > 0 then out.notes_in = { table.unpack(crew.notes) } end
  local templates, checks = {}, {}
  for _, t in ipairs(crew.templates or {}) do templates[#templates + 1] = { id = t[1], text = t[2] } end
  for _, k in ipairs(crew.checks or {}) do checks[#checks + 1] = { id = k[1], text = k[2] } end
  if #templates > 0 then out.templates = templates end
  if #checks > 0 then out.checks = checks end
  -- (what the driver has open, and what is ticked and typed so far)
  if open == "note" then
    out.open = "note"
    out.note = { template = note.template, text = note.text }
  elseif open == "check" then
    out.open = "check"
    local ticked = {}
    for id in pairs(check.ticked) do ticked[#ticked + 1] = id end
    table.sort(ticked)
    out.check = { ticked = #ticked > 0 and ticked or nil, text = check.text }
  end
  return out
end

-- the strip's facts now
function M.facts()
  return facts_at(minute())
end

-- the strip's elements for its facts `f`
function M.rows(f)
  local children = {}
  local function add(x) children[#children + 1] = x end
  add({ type = "row", children = {
    { type = "icon", name = "badge", color = ACCENT },
    { type = "text", text = f.piece.text, weight = "bold", color = TONE[f.piece.tone], grow = true },
  } })
  local b = f.next_break
  if b then
    local when = b.under_way and tr("now, until %{time}", { time = b.to }) or (b.from .. " · " .. until_text(b.left))
    add(line("coffee", tr("Next break"), b.at .. "  " .. when))
  end
  local r = f.next_relief
  if r then
    add(line("swap_horiz", tr("Next relief"), r.at .. "  " .. r.from .. " · " .. until_text(r.left), r.soon and AMBER or nil))
  end
  if f.ends then add(line("flag", tr("End of duty"), f.ends.from .. "  " .. f.ends.at)) end
  for _, ch in ipairs(f.changes or {}) do
    add({ type = "text", text = "• " .. ch, size = 12, color = AMBER })
  end
  local fu = f.fuel
  if fu then
    local label = fu.kind == "charge" and tr("Charge") or tr("Fuel")
    add({ type = "row", children = {
      { type = "icon", name = fu.kind == "charge" and "battery_charging_full" or "local_gas_station", color = fu.low and RED or DIM, size = 16 },
      { type = "text", text = label .. " " .. math.floor(fu.level * 100 + 0.5) .. " %", size = 12, color = fu.low and RED or DIM, wrap = false },
      { type = "bar", value = fu.level, grow = true, color = fu.low and RED or GREEN },
    } })
    if fu.to_depot then add({ type = "text", text = tr("Return to the depot to refuel"), size = 12, color = RED }) end
  end
  if open == "note" then
    note_form(children)
  elseif open == "check" then
    check_form(children)
  else
    add({ type = "row", children = {
      { type = "button", id = "strip_note", text = tr("Leave a note"), icon = "sticky_note_2", grow = true },
      { type = "button", id = "strip_check", text = tr("Defect check"), icon = "build", grow = true },
    } })
  end
  return children
end

-- the relief coming within the warning time, said once
local function warn_relief(f)
  local r = f.next_relief
  if not (r and r.soon) then return end
  local key = r.kind .. r.from
  if told[key] then return end
  told[key] = true
  notify("swap_horiz", tr("Relief due"), tr("At %{place} at %{time}", { place = r.at, time = r.from }))
end

-- the strip now (`on`: the driver is at work on the company's duty)
function M.update(on)
  at_work = on == true
  if not (crew and omsi.ui and omsi.ui.set) then return end
  local f = (enabled and on) and facts_at(minute()) or nil
  if not f then
    navigator.put("strip", nil)
    if shown then
      layout.remove(PANEL)
      shown = false
    end
    return
  end
  warn_relief(f)
  if navigator.docks() then return navigator.put("strip", M.rows(f)) end
  layout.set(PANEL, { anchor = "bottom_left", x = 16, y = 16, width = 340, accent = ACCENT, draggable = true, children = M.rows(f) })
  shown = true
end

-- --- sign-on and sign-off -----------------------------------------------------------------------

-- the driver signed on (or is in the bus at the duty's start): the notes left on the bus, once
function M.signed_on()
  if not crew then return end
  omsi.data.strip_seen = omsi.data.strip_seen or {}
  for _, n in ipairs(crew.notes or {}) do
    if not omsi.data.strip_seen[n] then
      omsi.data.strip_seen[n] = true
      notify("sticky_note_2", tr("A note was left for you"), n)
    end
  end
end

-- the driver signs off (or the duty ended): the walk-around opens, once
function M.sign_off()
  if not crew then return false end
  if not check.done then
    open = "check"
    M.update(true)
  end
  return true
end

-- a text cut to the longest the company allows, at the edge of a character
local function cut(text)
  local n = math.tointeger(tonumber(crew and crew.text_max)) or 140
  if #text <= n then return text end
  local i = n
  while i > 0 do
    local b = text:byte(i + 1)
    if b and b >= 0x80 and b < 0xC0 then i = i - 1 else break end
  end
  return text:sub(1, i)
end

-- the handover note typed, left for the next driver
local function send_note()
  if not (note.template or note.text ~= "") then return false end
  report("note", { template = note.template or "", text = note.text, bus_id = crew.bus })
  notify("sticky_note_2", tr("Note left"), tr("The next driver of bus %{bus} sees it", { bus = crew.bus_number or "" }))
  note = { template = nil, text = "" }
  open = nil
  return true
end

-- a note left from the on-board computer; false and why when it was not
function M.note(text)
  if type(text) ~= "string" then return false, "bad_command" end
  if not crew then return false, "not_now" end
  note = { template = nil, text = cut(text) }
  if not send_note() then return false, "empty" end
  M.update(true)
  return true
end

local function send_check(all_fine)
  local sent = 0
  if not all_fine then
    for _, k in ipairs(crew.checks or {}) do
      if check.ticked[k[1]] then
        report("defect", { defect = k[1], text = check.text, bus_id = crew.bus })
        sent = sent + 1
      end
    end
    if sent == 0 and check.text ~= "" then
      report("defect", { defect = "other", text = check.text, bus_id = crew.bus })
      sent = 1
    end
  end
  check = { ticked = {}, text = "", done = true }
  open = nil
  if sent > 0 then
    notify("build", tr("Defects reported"), tr("%{n} reported to the workshop", { n = sent }))
  end
end

-- the walk-around done from the on-board computer: `ok` all in order, else `items` (check ids)
-- and `text` are the defects
function M.check(ok, items, text)
  if type(ok) ~= "boolean" or type(items) ~= "table" or type(text) ~= "string" then return false, "bad_command" end
  if not crew then return false, "not_now" end
  check = { ticked = {}, text = cut(text), done = false }
  for _, id in ipairs(items) do
    if type(id) == "string" then check.ticked[id] = true end
  end
  send_check(ok)
  M.update(true)
  return true
end

-- a click on the strip: whether it was the strip's
function M.click(p, element)
  element = element or ""
  if (p ~= PANEL and not (navigator.is(p) and element:sub(1, 6) == "strip_")) or not crew then return false end
  if element == "strip_note" then open = "note"
  elseif element == "strip_check" then open = "check"
  elseif element == "strip_close" then open = nil
  elseif element == "strip_note_send" or element == "strip_note_text" then
    send_note()
  elseif element == "strip_chk_send" then send_check(false)
  elseif element == "strip_chk_ok" then send_check(true)
  end
  M.update(true)
  return true
end

-- a checkbox ticked or a field typed into: whether it was the strip's
function M.change(p, element, value)
  element = element or ""
  if (p ~= PANEL and not (navigator.is(p) and element:sub(1, 6) == "strip_")) or not crew then return false end
  local tpl = element:match("^strip_tpl_(.+)$")
  local chk = element:match("^strip_chk_(.+)$")
  if tpl then
    note.template = value and tpl or nil
  elseif element == "strip_note_text" then
    note.text = tostring(value or "")
  elseif element == "strip_chk_text" then
    check.text = tostring(value or "")
  elseif chk then
    check.ticked[chk] = value and true or nil
  else
    return false
  end
  return true
end

function M.stop()
  if shown and omsi.ui and omsi.ui.remove then layout.remove(PANEL) end
  shown = false
end

return M
