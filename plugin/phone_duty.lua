-- Omsi-Hub's plugin: the phone's Duty tab (nav.lua) - the company's duty: signing on where
-- Omsi-Hub asks for it, the duty order, starting it and signing off (main.lua's company panel,
-- `hooks.duty_children`), the duty strip (the next break, the relief, the end, today's changes,
-- the notes and the walk-around: duty_strip.lua) and the duty's trips, the one under way marked.
-- Its modes: a driver still to sign on, and the walk-around at sign-off.

local signon = require("signon")
local strip = require("duty_strip")
local P = require("phone_theme")

local M = {}

M.id = "duty"
M.title = "Duty"
M.icon = "badge"

-- the duty's trips: done, under way, to come
local function trip_list(ctx)
  local th, tr, d = ctx.theme, ctx.tr, ctx.duty
  if #ctx.trips == 0 then return { P.text(th, tr("No trips yet: start your duty"), "dim") } end
  local rows = {}
  for k, t in ipairs(ctx.trips) do
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
  if not ctx.can.box then
    -- (an openOMSI before coloured cells writes a cell's table as nothing: plain texts there)
    for _, r in ipairs(rows) do
      for i, cell in ipairs(r) do r[i] = cell.text end
    end
  end
  return { { type = "table", columns = { "", tr("Line"), tr("Dep."), "", tr("Arr.") }, widths = { 0.08, 0.14, 0.16, 0.46, 0.16 },
    aligns = { "center", "left", "left", "left", "right" }, rows = rows, size = 12.5, color = th.ink } }
end

function M.rows(ctx)
  local out = ctx.hooks.duty_children and ctx.hooks.duty_children() or {}
  if ctx.at_work then
    local s = strip.children()
    if #s > 0 then
      out[#out + 1] = { type = "divider" }
      for _, e in ipairs(s) do out[#out + 1] = e end
    end
  end
  if ctx.duty then
    out[#out + 1] = { type = "divider" }
    for _, e in ipairs(trip_list(ctx)) do out[#out + 1] = e end
  end
  if #out == 0 then out[1] = P.text(ctx.theme, ctx.tr("No duty: take one in the Omsi-Hub app"), "dim") end
  return out
end

-- signing on, and the walk-around at sign-off: the tab is about nothing else then
function M.mode(ctx)
  if ctx.hooks.duty_children and not signon.at_work() then return ctx.hooks.duty_children() end
  if strip.form() == "check" then return strip.children() end
  return nil
end

function M.slim(ctx)
  if ctx.hooks.duty_children and not signon.at_work() then return P.text(ctx.theme, ctx.tr("Not signed on"), "ink", 12.5, "bold") end
  return nil
end

return M
