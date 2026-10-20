-- Omsi-Hub's plugin: the phone's Bus tab (nav.lua) - the bus the driver is in: the fuel left
-- (and back to the depot when it runs low), the passengers seated and standing.

local P = require("phone_theme")

local M = {}

M.id = "bus"
M.title = "Bus"
M.icon = "directions_bus"

-- (under this much of the tank: back to the depot)
local LOW = 0.15
-- (the most fuel seen since the drive began: the tank's size where the bus does not say it)
local most = 0

-- the fuel left, 0 to 1 (nil: the bus does not tell)
local function fuel(ctx)
  if not ctx.has("bus.fuel") then return nil end
  local litres = tonumber(omsi.bus.fuel())
  if not litres then return nil end
  most = math.max(most, litres)
  if most <= 0 then return nil end
  return math.min(litres / most, 1)
end

function M.rows(ctx)
  local th, tr = ctx.theme, ctx.tr
  local out = {}
  local level = fuel(ctx)
  if level then
    local low = level < LOW
    out[#out + 1] = { type = "row", gap = 8, children = {
      { type = "icon", name = "local_gas_station", size = 16, color = th.dim },
      { type = "text", text = tr("Fuel"), size = 13, color = th.ink, grow = true, wrap = false },
      { type = "text", text = math.floor(level * 100 + 0.5) .. " %", size = 13, weight = "bold", color = th.ink, wrap = false },
    } }
    out[#out + 1] = { type = "bar", value = level, height = 6, color = th.ink, background = th.track }
    if low then out[#out + 1] = P.text(th, tr("Return to the depot to refuel"), "ink", 12, "bold") end
  end
  if ctx.passengers then
    out[#out + 1] = { type = "row", gap = 6, children = {
      P.stat(th, tr("Seated"), tostring(ctx.seated or ctx.passengers), nil, ctx.can.box),
      P.stat(th, tr("Standing"), tostring(ctx.standing or 0), nil, ctx.can.box),
    } }
  end
  if #out == 0 then out[1] = P.text(th, tr("The bus tells nothing of itself"), "dim") end
  return out
end

return M
