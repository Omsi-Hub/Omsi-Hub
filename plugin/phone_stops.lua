-- Omsi-Hub's plugin: the phone's Stops tab (nav.lua) - every stop of the trip still to come,
-- with its time (and, off the timetable, when the bus will be there); as many as the room
-- under the map has room for, and how many more to the terminus.

local board = require("board")
local P = require("phone_theme")

local M = {}

M.id = "stops"
M.title = "Stops"
M.icon = "departure_board"
M.fit = { 8, 64 }

function M.rows(ctx)
  if not ctx.duty then return { P.text(ctx.theme, ctx.tr("No trip under way"), "dim") } end
  local rows = board.rows(ctx.duty, ctx.trips, ctx.stops, ctx.clock, ctx.fit, not ctx.can.stops, 128)
  -- (the status line has the badge, the terminus and the chip: the board's own first row goes)
  table.remove(rows, 1)
  return rows
end

return M
