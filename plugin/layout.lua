-- Omsi-Hub's plugin: the size and the place of its panels, the driver's own. While the panels
-- have the mouse (F10) each of them shows a row under its content: smaller, its size, larger,
-- and back to how it came. The size multiplies the panel's sizes (its width, padding and gaps,
-- its texts, icons, bars and charts; openOMSI's badges and buttons keep theirs). A panel the
-- driver dragged keeps its place in later drives: where it was dragged to is kept with its
-- size in `omsi.data.layout`, and given as the panel's offset when it comes on the screen again.
--
-- Where the game lets the player resize a panel with the mouse (openOMSI's ui version 6:
-- `omsi.ui.resized`, the `ui_resize` event), a panel that can be dragged can be resized too, at
-- its edges and corners: its width and height (`resizable`, between its `min_`/`max_` sizes) are
-- the driver's, kept with its place and given back as its `width` and `height` in later drives
-- (the content laid out from the top and cut at the bottom, as the game does with a height). The
-- size row stays: it sizes the texts with the panel. An older game knows none of these keys and
-- skips them; it keeps the size row alone.
--
-- The board in the navigator (`anchor = "navigator"`) is the navigator's: it goes as it is, at
-- the navigator's size and place, never resizable.

local M = {}

local MIN, MAX, STEP = 0.6, 2, 0.1
-- (the defaults of openOMSI's panel table, and of its elements, that a size multiplies)
local PANEL_SIZES = { width = 300, padding = 12, gap = 6, radius = 12 }
local SIZES = {
  text = { size = 14 }, icon = { size = 20 }, row = { gap = 8 }, box = { gap = 4 },
  space = { size = 8 }, bar = { height = 6 }, chart = { height = 48 }, image = { width = 64, height = 64 },
  stops = { size = 13 }, table = { size = 13 },
}
-- (an anchor's place across and down: 0 the left or top, 1 the right or bottom)
local FACTORS = {
  top_left = { 0, 0 }, top = { 0.5, 0 }, top_right = { 1, 0 },
  left = { 0, 0.5 }, center = { 0.5, 0.5 }, right = { 1, 0.5 },
  bottom_left = { 0, 1 }, bottom = { 0.5, 1 }, bottom_right = { 1, 1 },
}
local DIM = "#959DB0"
-- (a resizable panel's bounds against its own width: as narrow as this share of it, as wide as
-- this many times it; its height from this many points (at its size) to this many)
local NARROWEST, WIDEST = 0.6, 2.5
local LOWEST, HIGHEST = 80, 1600

local tr = function(t) return t end
-- (by panel: the table it was last given, and the offset it came on the screen with)
local last, base = {}, {}
-- (the panels whose size the player gave them the game is to forget at their next setting, and
-- that size: not taken for the player's again)
local forget, stale = {}, {}

function M.setup(translate)
  tr = translate
end

local function kept(id)
  omsi.data.layout = type(omsi.data.layout) == "table" and omsi.data.layout or {}
  local k = omsi.data.layout[id]
  if type(k) ~= "table" then
    k = { scale = 1, dx = 0, dy = 0 }
    omsi.data.layout[id] = k
  end
  k.scale = math.min(math.max(tonumber(k.scale) or 1, MIN), MAX)
  k.dx, k.dy = tonumber(k.dx) or 0, tonumber(k.dy) or 0
  -- (the size the player gave it with the mouse: none, or both)
  k.width, k.height = tonumber(k.width), tonumber(k.height)
  if not (k.width and k.height and k.width > 0 and k.height > 0) then k.width, k.height = nil, nil end
  return k
end

-- how far the player dragged the panel `id` on the screen now (nil: it is not there)
local function dragged(id)
  if not (omsi.ui and omsi.ui.moved) then return 0, 0 end
  local dx, dy = omsi.ui.moved(id)
  if dx == nil then return nil end
  return tonumber(dx) or 0, tonumber(dy) or 0
end

-- whether the game lets the player resize panels with the mouse (ui version 6)
local function resizes()
  return omsi.ui ~= nil and type(omsi.ui.resized) == "function"
end

-- the size the player gave the panel `id` with the mouse this session (nil: none)
local function resized(id)
  if not resizes() then return nil end
  local w, h = omsi.ui.resized(id)
  w, h = tonumber(w), tonumber(h)
  if not (w and h and w > 0 and h > 0) then return nil end
  return w, h
end

local function focused()
  return omsi.ui and omsi.ui.focused and omsi.ui.focused() or false
end

-- an element (and the ones in it) at the size `k`
local function sized(e, k)
  if type(e) ~= "table" then return e end
  local out = {}
  for key, v in pairs(e) do out[key] = v end
  for key, default in pairs(SIZES[e.type] or {}) do out[key] = (tonumber(e[key]) or default) * k end
  if type(e.children) == "table" then
    local children = {}
    for i, c in ipairs(e.children) do children[i] = sized(c, k) end
    out.children = children
  end
  return out
end

-- the panel's table at the size `k`, moved by (dx, dy) on the screen
function M.sized(t, k, dx, dy)
  local out = {}
  for key, v in pairs(t) do out[key] = v end
  for key, default in pairs(PANEL_SIZES) do out[key] = (tonumber(t[key]) or default) * k end
  local children = {}
  for i, c in ipairs(t.children or {}) do children[i] = sized(c, k) end
  out.children = children
  local f = FACTORS[t.anchor or "top_left"] or FACTORS.top_left
  out.x = (tonumber(t.x) or 16) + (f[1] == 1 and -dx or dx)
  out.y = (tonumber(t.y) or 16) + (f[2] == 1 and -dy or dy)
  return out
end

-- the panel's table `shown` resizable with the mouse, at the size the driver gave it (`k` its
-- kept layout); bounds of its own (`min_width` ...) stay
local function sizable(shown, k)
  local wide = tonumber(shown.width) or PANEL_SIZES.width
  shown.resizable = true
  shown.min_width = shown.min_width or math.floor(wide * NARROWEST)
  shown.max_width = shown.max_width or math.floor(wide * WIDEST)
  shown.min_height = shown.min_height or math.floor(LOWEST * k.scale)
  shown.max_height = shown.max_height or HIGHEST
  if k.width then shown.width = math.min(math.max(k.width, shown.min_width), shown.max_width) end
  if k.height then shown.height = math.min(math.max(k.height, shown.min_height), shown.max_height) end
  return shown
end

-- the size the driver gave the panel `id` this session, kept with it
local function keep_size(id)
  local w, h = resized(id)
  if not w or (stale[id] and stale[id][1] == w and stale[id][2] == h) then return end
  local k = kept(id)
  k.width, k.height = w, h
end

local function controls(id, k)
  return {
    { type = "divider" },
    { type = "row", gap = 6, children = {
      { type = "icon", name = "open_with", size = 16, color = DIM },
      { type = "text", text = tr("Size"), size = 12, weight = "bold", color = DIM, grow = true, wrap = false },
      { type = "button", id = "hub_size_down", icon = "remove", visible = k > MIN + 1e-6 },
      { type = "text", id = "hub_size", text = string.format("%d %%", math.floor(k * 100 + 0.5)), size = 12, weight = "bold", align = "center" },
      { type = "button", id = "hub_size_up", icon = "add", visible = k < MAX - 1e-6 },
      { type = "button", id = "hub_size_reset", icon = "restart_alt" },
    } },
  }
end

-- Puts the panel `id` on the screen at the driver's size and place (`t`: its table, as
-- `omsi.ui.set` takes it).
function M.set(id, t)
  last[id] = t
  if t.anchor == "navigator" then return omsi.ui.set(id, t) end
  local k = kept(id)
  local dx, dy = dragged(id)
  if dx == nil or base[id] == nil then
    -- (on the screen again: where the driver left it last)
    base[id] = { k.dx, k.dy }
    dx, dy = 0, 0
  end
  k.dx, k.dy = base[id][1] + dx, base[id][2] + dy
  local shown = M.sized(t, k.scale, base[id][1], base[id][2])
  if resizes() and t.draggable then
    keep_size(id)
    sizable(shown, k)
    -- (the game keeps the size the player gave a panel until it is told the panel is not
    -- resizable: so told once, for the size of the size row or of the reset)
    if forget[id] then
      forget[id] = nil
      local off = {}
      for key, v in pairs(shown) do off[key] = v end
      off.resizable = false
      omsi.ui.set(id, off)
    end
  end
  if focused() and t.draggable then
    local children = {}
    for i, c in ipairs(shown.children) do children[i] = c end
    for _, c in ipairs(controls(id, k.scale)) do children[#children + 1] = c end
    shown.children = children
  end
  return omsi.ui.set(id, shown)
end

-- Takes the panel `id` off the screen; where it was dragged to is kept for the next time.
function M.remove(id)
  if last[id] and base[id] then
    local dx, dy = dragged(id)
    if dx then
      local k = kept(id)
      k.dx, k.dy = base[id][1] + dx, base[id][2] + dy
    end
    keep_size(id)
  end
  last[id], base[id] = nil, nil
  if omsi.ui and omsi.ui.remove then return omsi.ui.remove(id) end
  return false
end

-- Sets every panel again (the panels got or gave back the mouse: the size row comes or goes).
local was_focused = false
function M.refresh()
  was_focused = focused()
  for id, t in pairs(last) do M.set(id, t) end
end

-- The panels got or gave back the mouse some other way (Esc, a menu of the game): set again.
function M.watch()
  if focused() ~= was_focused then M.refresh() end
end

-- Keeps where every panel was dragged to, and the size it was given (the game ends, the plugin
-- stops).
function M.keep()
  for id in pairs(last) do
    local dx, dy = dragged(id)
    if dx and base[id] then
      local k = kept(id)
      k.dx, k.dy = base[id][1] + dx, base[id][2] + dy
    end
    keep_size(id)
  end
end

-- The player resized the panel `id` with the mouse (the `ui_resize` event: its new width and
-- height): kept, and the panel set again at it (its content may follow the height). Whether it
-- was given a size.
function M.resize(id, width, height)
  local w, h = tonumber(width), tonumber(height)
  if not (type(id) == "string" and w and h and w > 0 and h > 0) then return false end
  local k = kept(id)
  k.width, k.height = w, h
  stale[id] = nil
  if last[id] then M.set(id, last[id]) end
  return true
end

-- The size the driver gave the panel `id`: its width, its height and its size row's scale (nil:
-- none, or a game that does not resize panels).
function M.size(id)
  if not resizes() then return nil end
  local k = kept(id)
  if not (k.width and k.height) then return nil end
  return k.width, k.height, k.scale
end

-- A panel the plugin sets itself (`t`, one not dragged: the trip's card) resizable where the
-- game can, at the size the driver gave it.
function M.resizable(id, t)
  if not resizes() then return t end
  keep_size(id)
  local shown = {}
  for key, v in pairs(t) do shown[key] = v end
  return sizable(shown, kept(id))
end

-- A click on a panel's size row: handled (true) or not the layout's.
function M.click(id, element)
  if type(element) ~= "string" or not element:match("^hub_size") or not last[id] then return false end
  local k = kept(id)
  keep_size(id)
  local before = k.scale
  if element == "hub_size_down" then
    k.scale = math.max(k.scale - STEP, MIN)
  elseif element == "hub_size_up" then
    k.scale = math.min(k.scale + STEP, MAX)
  elseif element == "hub_size_reset" then
    -- (back where its table puts it: the drag of this session undone with the offset, and the
    -- size the player gave it)
    local dx, dy = dragged(id)
    k.scale = 1
    base[id] = { -(dx or 0), -(dy or 0) }
    k.width, k.height = nil, nil
  end
  k.scale = math.floor(k.scale * 10 + 0.5) / 10
  -- (a size the player gave it grows and shrinks with the size row)
  if k.width and k.height and before > 0 then
    k.width, k.height = k.width * k.scale / before, k.height * k.scale / before
  end
  if resizes() then
    local w, h = resized(id)
    forget[id], stale[id] = true, w and { w, h } or nil
  end
  M.set(id, last[id])
  return true
end

return M
