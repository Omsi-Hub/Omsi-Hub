-- Omsi-Hub's plugin: its notifications - the company's notices, service control's messages, the
-- drive watch's offences, the duty's parts and reliefs, the on-time alerts - kept for the
-- on-board computer (Omsi-Hub's window, through the companion's channel: `list`), the newest
-- first, each closed there by the driver (`close`); a few at a time, the older ones behind them
-- until those are gone. Each goes by itself after a while (the driver may be looking at the
-- road); one that must be read (`sticky`: an urgent message, an alert) stays until it is closed.
-- Nothing is drawn in the game.

local M = {}

-- (the first few run their time - the on-board computer shows them -; kept in all, the oldest
-- going first)
local SHOWN, KEPT = 3, 8

local list = {}
local seq = 0

-- `text` told the driver; `opts`: title, icon, color, seconds (about how long it is worth
-- showing), sticky. Its number (to `close` it).
function M.say(text, opts)
  opts = opts or {}
  seq = seq + 1
  table.insert(list, 1, {
    id = seq, text = text or "", title = opts.title or "", icon = opts.icon or "notifications", color = opts.color or "#F58620",
    left = math.max((tonumber(opts.seconds) or 8) * 3, 20), sticky = opts.sticky == true,
  })
  while #list > KEPT do table.remove(list) end
  return seq
end

-- every second: the ones whose time is up go
function M.tick(dt)
  for k = #list, 1, -1 do
    local n = list[k]
    -- (only those the driver sees run their time; those behind wait their turn)
    if k <= SHOWN and not n.sticky then
      n.left = n.left - (dt or 1)
      if n.left <= 0 then table.remove(list, k) end
    end
  end
end

-- the notification `id` put away; whether there was one
function M.close(id)
  local found = false
  for k, n in ipairs(list) do
    if n.id == id then
      table.remove(list, k)
      found = true
      break
    end
  end
  return found
end

-- the notifications now, newest first, as data (for the on-board computer): `n` its number,
-- title, text, icon, colour and whether it stays until it is closed
function M.list()
  local out = {}
  for k, n in ipairs(list) do
    out[k] = { n = n.id, title = n.title:sub(1, 200), text = n.text:sub(1, 2000), icon = n.icon, color = n.color, sticky = n.sticky }
  end
  return out
end

return M
