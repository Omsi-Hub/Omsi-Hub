-- Omsi-Hub's plugin: the phone's Bus tab (nav.lua) - the bus the driver is in: the fuel or the
-- charge left (and back to the depot when it runs low), the passengers seated and standing, the
-- tickets sold and the fares taken today, the collisions and how long their repair takes, the
-- bus's open defects at the company's workshop, and the handover notes: those left for the
-- driver, and one of their own for the next driver (duty_strip.lua's note; typing needs F10).

local strip = require("duty_strip")
local summary = require("phone_summary")
local P = require("phone_theme")

local M = {}

M.id = "bus"
M.title = "Bus"
M.icon = "directions_bus"

-- (under this much of the tank or the battery: back to the depot, unless the company says)
local LOW = 0.15
-- (the most fuel seen since the drive began: the tank's size where the bus does not say it)
local most = 0
-- (the notes and the defects listed at most)
local NOTES, DEFECTS = 4, 6

-- a text from elsewhere cut to `most` characters (the game refuses a panel with a longer one)
local function cut(s, most)
  s = tostring(s or "")
  local n = utf8.len(s)
  if not n then return s:sub(1, most) end
  if n <= most then return s end
  return s:sub(1, utf8.offset(s, most) - 1) .. "…"
end

-- the company's crew data of the duty (nil: none)
local function crew(ctx)
  local a = ctx.assignment
  return type(a) == "table" and type(a.crew) == "table" and a.crew or nil
end

-- the fuel or the charge left, 0 to 1, and which ("fuel", "charge"); nil where the bus does
-- not tell
local function level(ctx)
  if ctx.has("bus.get_vars") then
    local v = omsi.bus.get_vars({ "elec_battery_soc", "battery_soc", "soc", "engine_tank_capacity" })
    if type(v) == "table" then
      for _, k in ipairs({ "elec_battery_soc", "battery_soc", "soc" }) do
        local x = tonumber(v[k])
        if x then return x > 1 and math.min(x / 100, 1) or math.max(x, 0), "charge" end
      end
      local cap = tonumber(v.engine_tank_capacity)
      if cap and cap > 0 then most = math.max(most, cap) end
    end
  end
  if not ctx.has("bus.fuel") then return nil end
  local litres = tonumber(omsi.bus.fuel())
  if not litres then return nil end
  most = math.max(most, litres)
  if most <= 0 then return nil end
  return math.min(litres / most, 1), "fuel"
end

-- a line: its icon (in the accent), its text and on the right a value
local function line(th, icon, text, value)
  local children = {
    { type = "icon", name = icon, size = 16, color = th.accent },
    { type = "text", text = text, size = 13, color = th.ink, grow = true },
  }
  if value then children[3] = { type = "text", text = value, size = 13, weight = "bold", color = th.ink, wrap = false } end
  return { type = "row", gap = 8, children = children }
end

-- the fuel or the charge, its bar, and the depot when it runs low
local function fuel(ctx, out)
  local th, tr = ctx.theme, ctx.tr
  local left, kind = level(ctx)
  if not left then return end
  local c = crew(ctx)
  local low = left < (c and tonumber(c.low_fuel) or LOW)
  -- (the tank on a card: the blue, the time's red when it runs low)
  local tone = low and th.late or th.primary
  local card = {
    line(th, kind == "charge" and "bolt" or "local_gas_station", kind == "charge" and tr("Charge") or tr("Fuel"), math.floor(left * 100 + 0.5) .. " %"),
    { type = "bar", value = left, height = 6, color = low and th.late or th.accent, background = th.track },
  }
  if low then card[#card + 1] = P.text(th, kind == "charge" and tr("Return to the depot to charge") or tr("Return to the depot to refuel"), "ink", 12.5, "bold") end
  P.card(out, th, ctx.can.box, card, tone, 6)
end

-- the tickets sold and the money taken this session (nil: the game does not say)
local function sales(ctx)
  if not ctx.has("bus.sales") then return nil end
  local tickets, money = omsi.bus.sales()
  if not tonumber(money) then return nil end
  return tonumber(tickets) or 0, tonumber(money)
end

-- the collisions and the repair they need (where the bus has any)
local function damage(ctx, out)
  if not ctx.has("bus.damage") then return end
  local d = omsi.bus.damage()
  local n = type(d) == "table" and tonumber(d.crashes) or nil
  if not n or n <= 0 then return end
  local m = tonumber(d.repair_minutes)
  P.card(out, ctx.theme, ctx.can.box, { line(ctx.theme, "warning", ctx.tr("Collisions") .. " " .. math.floor(n), m and m > 0 and ctx.tr("repair %{m} min", { m = math.ceil(m) }) or nil) }, ctx.theme.late)
end

-- the bus's open defects (the company's: `crew.defects`, each { part, label, text, since })
local function defects(ctx, out)
  local c = crew(ctx)
  local list = c and type(c.defects) == "table" and c.defects or {}
  if #list == 0 then return end
  local th = ctx.theme
  -- (a part's name as the walk-around's checks call it)
  local names = {}
  for _, k in ipairs(type(c.checks) == "table" and c.checks or {}) do
    if type(k) == "table" and k[1] then names[k[1]] = k[2] end
  end
  out[#out + 1] = P.heading(th, ctx.tr("Open defects"))
  for k = 1, math.min(#list, DEFECTS) do
    local x = list[k]
    if type(x) == "table" then
      local what = tostring((x.label or "") ~= "" and x.label or names[x.part] or x.part or "")
      local text = (x.text or "") ~= "" and (what ~= "" and (what .. ": " .. tostring(x.text)) or tostring(x.text)) or what
      out[#out + 1] = line(th, "construction", cut(text, 200), (x.since or "") ~= "" and cut(x.since, 20) or nil)
    end
  end
end

-- the notes left on the bus, and one for the next driver (duty_strip.lua's form, its own
-- elements in the phone's look: the clicks go to it)
local function notes(ctx, out)
  local c = crew(ctx)
  if not c then return end
  local th, tr = ctx.theme, ctx.tr
  local left = type(c.notes) == "table" and c.notes or {}
  if #left > 0 then
    out[#out + 1] = P.heading(th, tr("Notes for you"))
    for k = 1, math.min(#left, NOTES) do out[#out + 1] = line(th, "description", cut(left[k], 300)) end
  end
  if #strip.children() == 0 then return end
  if strip.form() == "note" then
    for _, e in ipairs(P.adopt(th, strip.tools())) do out[#out + 1] = e end
    out[#out + 1] = P.text(th, tr("Typing needs F10"), "dim", 11.5)
  else
    out[#out + 1] = { type = "button", id = "strip_note", text = tr("Leave a note"), icon = "description", color = th.secondary }
  end
end

function M.rows(ctx)
  local th, tr = ctx.theme, ctx.tr
  local out = {}
  fuel(ctx, out)
  if ctx.passengers then
    out[#out + 1] = { type = "row", gap = 6, children = {
      P.stat(th, tr("Seated"), tostring(math.tointeger(ctx.seated or ctx.passengers) or 0), nil, ctx.can.box),
      P.stat(th, tr("Standing"), tostring(math.tointeger(ctx.standing or 0) or 0), nil, ctx.can.box),
    } }
  end
  local tickets, money = sales(ctx)
  if money then
    out[#out + 1] = { type = "row", gap = 6, children = {
      P.stat(th, tr("Tickets sold"), tostring(math.tointeger(tickets) or tickets), nil, ctx.can.box),
      P.stat(th, tr("Fares taken"), summary.money(ctx.assignment, money), nil, ctx.can.box),
    } }
  end
  damage(ctx, out)
  defects(ctx, out)
  notes(ctx, out)
  if #out == 0 then out[1] = P.text(th, tr("The bus tells nothing of itself"), "dim") end
  return out
end

-- the slim phone: the depot, when the fuel runs low
function M.slim(ctx)
  local left, kind = level(ctx)
  local c = crew(ctx)
  if not left or left >= (c and tonumber(c.low_fuel) or LOW) then return nil end
  local th, tr = ctx.theme, ctx.tr
  return line(th, kind == "charge" and "bolt" or "local_gas_station", kind == "charge" and tr("Return to the depot to charge") or tr("Return to the depot to refuel"), math.floor(left * 100 + 0.5) .. " %")
end

return M
