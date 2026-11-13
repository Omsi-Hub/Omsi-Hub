-- Omsi-Hub's plugin: the time each call into it takes.
--
-- openOMSI gives every call into a plugin (one event handler, one timer) 50 ms. Past them it
-- stops the plugin and switches it off until the game restarts ("ran longer than 50 ms and was
-- stopped"), and the time the game spends inside its own functions counts too: reading the
-- traffic, giving the AI its trips, writing a file. Its watch looks at the clock every 10,000
-- Lua instructions, so a call is stopped only where Lua runs on after the 50 ms; a call that
-- ends just after a slow function of the game's goes through.
--
-- So the plugin keeps its calls short: a job reads what it needs from the game in one call,
-- works it out in the next and writes it in the one after; a long list is done in portions
-- (`over`). And each call is timed: one that took long is written into the game's log
-- (`[lua omsihub] slow: traffic 31 ms`), which Omsi-Hub copies into its own log - so that a
-- player's log says which part of the plugin came near the 50 ms.

local M = {}

-- a call that took this long is written into the log (ms)
M.SLOW = 20
-- at most this many such lines in a session
local MAX_NOTES = 40

-- the clock, in seconds (a test counts Lua instructions with it instead)
M.clock = type(os) == "table" and type(os.clock) == "function" and os.clock or nil
local started, depth = nil, 0
-- the longest call of each name, the longest written into the log, the lines written
local longest, worst, notes = {}, {}, 0

local function now()
  return M.clock and M.clock() or nil
end

-- whether the call under way has run for `ms` already (false where there is no clock)
function M.over(ms)
  local t = started and now()
  return t ~= nil and (t - started) * 1000 >= ms
end

local function note(name, ms)
  if ms > (longest[name] or 0) then longest[name] = ms end
  if ms < M.SLOW or ms <= (worst[name] or 0) * 1.25 or notes >= MAX_NOTES then return end
  worst[name] = ms
  notes = notes + 1
  if type(omsi.log) == "function" then omsi.log(string.format("slow: %s %d ms", name, math.floor(ms + 0.5))) end
end

-- `fn` as a call of its own, named `name`: timed (an event the game sends from inside it - a
-- duty started - is part of it)
function M.wrap(name, fn)
  return function(...)
    if depth == 0 then started = now() end
    depth = depth + 1
    local ok, r = pcall(fn, ...)
    depth = depth - 1
    if depth == 0 then
      local t = started and now()
      if t then note(name, (t - started) * 1000) end
      started = nil
    end
    -- (an error goes on as it came: openOMSI counts it, or stops the plugin for the time)
    if not ok then error(r, 0) end
    return r
  end
end

-- omsi.on and omsi.every with each call timed, under a name of its own: `every(seconds, fn,
-- name)`, `on(event, fn)` (the event's name); omsi.after's calls are "after"
function M.install()
  local on, every, after = omsi.on, omsi.every, omsi.after
  if type(on) == "function" then
    omsi.on = function(event, fn) return on(event, M.wrap(tostring(event), fn)) end
  end
  if type(every) == "function" then
    omsi.every = function(seconds, fn, name) return every(seconds, M.wrap(name or "every " .. tostring(seconds), fn)) end
  end
  if type(after) == "function" then
    omsi.after = function(seconds, fn) return after(seconds, M.wrap("after", fn)) end
  end
end

-- the longest call of each name so far (ms)
function M.longest()
  return longest
end

return M
