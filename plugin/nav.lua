-- Omsi-Hub's plugin: the phone unit (Luc, 2026-10-20) - openOMSI's navigator with Omsi-Hub's
-- part docked under its map (`anchor = "navigator"`): the map, the route, the next turn, the
-- clock and the next stop's card are the navigator's own; under them a status line (the trip's
-- line badge, where it goes, trip 3/8, how it stands against the timetable) and the tabs Ride,
-- Stops, Duty, Messages and Bus. Each tab is a module of its own (phone_*.lua, the registry
-- below); this one builds what they share once a second (`ctx`), draws the status line and the
-- tabs, and hands the clicks to them.
--
-- The panel is laid out 360 points wide at the navigator's scale, on the navigator's own card
-- (dark or light: the inks of `phone_theme.lua`, the light ones when the assignment says
-- `nav = { light = true }`); its background stays see-through, its buttons and tabs take the
-- mouse without F10 (`grab`, openOMSI's ui version 5). A short room under the map (`room` below
-- 150 points) gets the slim phone: the status line and the one line that matters now.
--
-- An openOMSI without the navigator's board (`omsi.ui.version` below 3 or no `omsi.ui.dock`),
-- the phone switched off (`nav = { phone = false }`), a navigator that shows no board for 6 s
-- (switched off, Shift+N put it away) or whose board another plugin has: the modules' own
-- panels, as before (the board, the trip's card, the inbox, the on-time card, the duty strip).
-- Never both: the modules are told (`dock`) whether the phone has them, and the board coming
-- back is looked for every frame (`poll`), so the phone takes them over in the same frame.
-- The slim phone leaves the inbox its own panel (it has no room to answer there).
--
-- A tab module (phone_ride, phone_stops, phone_duty, phone_msgs, phone_bus) exports:
--   M.id               "ride", "stops", "duty", "messages", "bus" (its element ids start with
--                      this and "_": "ride_...", "messages_..." - the clicks go by them)
--   M.title            its name in English ("Ride"); the shell puts it through tr()
--   M.tab              (optional) a shorter name for the tab itself, where the title is long
--   M.icon             a Material Symbols icon of the game's ("navigation", "forum", ...)
--   M.fit              (optional) { start, most }: how many entries of a list it shows at
--                      first and at most; the shell fits `ctx.fit` to the room by the height
--                      the panel came out
--   M.badge(ctx)       (optional) a count or a short text on its tab (nil or 0: none)
--   M.rows(ctx)        its content: a list of elements
--   M.mode(ctx)        (optional) a list of elements that takes its content over while it
--                      applies (sign-on, the break, the walk-around, trip done, duty done);
--                      as one begins, its tab comes up by itself (the driver may go elsewhere)
--   M.slim(ctx)        (optional) the one element it shows in the slim phone (nil: nothing)
--   M.click(ctx, id)   (optional) a click on one of its elements; true when it was its own
--   M.change(ctx, id, value) (optional) a checkbox, slider or text field; true when its own
--
-- ctx, built once a second (and again for a click) - every field may be nil where the game or
-- the drive has nothing:
--   tr(text, vars)     the plugin's texts in the player's language
--   theme              the inks (phone_theme.lua: ink, dim, faint, line, field, track, chip,
--                      badge, badge_ink, late, on_time, early, primary, secondary, neutral,
--                      tab_on, accent, light)
--   assignment         what Omsi-Hub gave (assignment.lua), nil on a drive of the player's own
--   hooks              main.lua's: duty_children(), parts(), boarded(), send(file, tag, value,
--                      append), report(kind, fields), issued(), say(text, title, icon, seconds)
--   has(name)          whether the game has the function ("bus.fuel", "ui.set_menu_item")
--   version            omsi.ui.version
--   can                { box, stops, segments, grab }: what this game's panels can draw
--                      (box/stops/segments from ui version 4, grab from 5)
--   duty               omsi.duty.get() (nil: no duty)
--   trips, stops       omsi.duty.trips(), omsi.duty.stops() ({} without a duty)
--   clock              the game's clock, seconds of the day
--   phase              "before", "running", "break", "finished" (board.phase; nil: no duty)
--   k, trip            the trip the status line is about (the next one during a break) and
--                      its entry of `trips`
--   line, terminus     that trip's line (duty.trips()[k].line, never duty.get().line) and
--                      its terminus as the driver reads it (phone_theme.terminus)
--   delay              seconds late (early below 0)
--   speed              km/h
--   passengers, seated, standing   on board now (omsi.bus.passengers)
--   room, height, width   the room under the map, the height the panel came to, 360 (points)
--   slim               room below 150 points: the slim phone
--   fit                the entries the tab shown may list (its M.fit, fitted to the room)
--   tab                the id of the tab shown
--   visible            the phone is on the screen this second (not while the board is away):
--                      only then is a message shown told read
--   unread             the control room's messages come in since the Messages tab was seen
--   wants              an instruction waits for the driver's answer (or a hold runs)
--   at_work            the driver is signed on (or no sign-on asked)
--   show(id)           brings the tab `id` up
-- The time colours and texts (+2:14 / −1:26 / on time, ±59 s like the navigator), hh:mm, the
-- line badge, the stats: phone_theme.lua.

local board = require("board")
local card = require("card")
local layout = require("layout")
local messages = require("messages")
local ontime = require("ontime")
local signon = require("signon")
local strip = require("duty_strip")
local P = require("phone_theme")

local M = {}

local PANEL = "omsihub_nav"
M.PANEL = PANEL
-- (the tabs element's id)
local TABS_ID = "phone_tab"

-- the tabs, in their order on the phone (the last in brackets: `require` gives two values)
local TABS = { require("phone_ride"), require("phone_stops"), require("phone_duty"), require("phone_msgs"), (require("phone_bus")) }
local BY_ID = {}
for k, t in ipairs(TABS) do BY_ID[t.id] = k end
-- (what the slim phone shows when no mode applies: the first tab of these with a line)
local SLIM_ORDER = { "ride", "duty", "stops", "bus" }

-- (shorter than this the slim phone; how long the board may stay away before the modules get
-- their own panels back, s - not counted while the game's menu is open, and long enough that a
-- look at the city map, which hides the board without a word, flips nothing)
local SLIM = 150
local GRACE = 6
local PADDING, GAP = 10, 6
-- (a tab with a count: about this many letters fit on it, the count's too)
local TAB_LETTERS = 8

local tr = function(t) return t end
local hooks = {}
local assignment = nil
local shown = false
local docked = false
local away = 0
-- (the room under the map and the height the panel came to, as last seen)
local size = { room = nil, height = 0 }
-- (by tab: the entries it lists, fitted to the room; whether its mode applied last time; the
-- height before its last entry was added, and the height that entry took where it did not fit)
local fit = {}
local moded = {}
local grew, need = {}, {}
local ctx = nil

omsi.data.nav_tab = BY_ID[omsi.data.nav_tab or ""] and omsi.data.nav_tab or "ride"

local function fn(name)
  local t = omsi
  for part in name:gmatch("[^%.]+") do
    if type(t) ~= "table" then return nil end
    t = t[part]
  end
  return type(t) == "function" and t or nil
end

local function has(name)
  return fn(name) ~= nil
end

local function version()
  return omsi.ui and tonumber(omsi.ui.version) or 0
end

-- `translate` the plugin's texts, `a` the assignment, `h` main.lua's hooks (see ctx.hooks)
function M.setup(translate, a, h)
  tr, assignment, hooks = translate, a, h or {}
  for _, t in ipairs(TABS) do fit[t.id] = t.fit and t.fit[1] or 4 end
end

-- whether this openOMSI has the navigator's board, and the phone is wanted
local function docks()
  if not (has("ui.set") and has("ui.dock") and version() >= 3) then return false end
  local nav = assignment and assignment.nav
  return not (type(nav) == "table" and nav.phone == false)
end

-- the board under the navigator's map: width, room, height, whether it is this plugin's
local function dock()
  local w, room, h, mine = omsi.ui.dock()
  if not w then return nil end
  return w, tonumber(room) or 0, tonumber(h) or 0, mine ~= false
end

local function theme()
  local nav = assignment and assignment.nav
  return P.theme(type(nav) == "table" and nav.light == true)
end

-- whether the phone is in the navigator now (the modules keep their own panels otherwise),
-- and whether it asks for the navigator's board at all (the board module then does not)
function M.active()
  return shown and docked
end

-- whether the room under the map gives the slim phone
function M.slim()
  return size.room ~= nil and size.room < SLIM
end

-- whether the inbox is in the phone (its own panel is the driver's else: left alone)
local function inbox_inside()
  return shown and docked and not M.slim()
end

function M.asking()
  return shown
end

-- the tab shown, and a tab brought up
function M.tab()
  return omsi.data.nav_tab
end

function M.show(id)
  if not BY_ID[id] then return end
  omsi.data.nav_tab = id
  if ctx then ctx.tab = id end
  if not inbox_inside() then return end
  if id == "messages" then messages.seen() else messages.hide() end
end

-- --- what the tabs share ------------------------------------------------------------------------

local function context()
  local d = has("duty.get") and omsi.duty.get() or nil
  local trips = d and has("duty.trips") and omsi.duty.trips() or {}
  local stops = d and has("duty.stops") and omsi.duty.stops() or {}
  local i = has("info") and omsi.info() or nil
  local now = type(i) == "table" and tonumber(i.clock) or 0
  local phase = d and board.phase(d, trips, now) or nil
  local k = d and (phase == "break" and d.trip + 1 or d.trip) or nil
  local trip = k and trips[k] or nil
  local total, seated, standing
  if has("bus.passengers") then total, seated, standing = omsi.bus.passengers() end
  local v = version()
  local room = size.room
  local tab = omsi.data.nav_tab
  return {
    tr = tr, theme = theme(), assignment = assignment, hooks = hooks, has = has,
    version = v, can = { box = v >= 4, stops = v >= 4, segments = v >= 4, grab = v >= 5 },
    duty = d, trips = trips, stops = stops, clock = now, phase = phase,
    k = k, trip = trip, line = trip and trip.line or "", terminus = trip and P.terminus(trip.line, trip.terminus) or "",
    delay = d and tonumber(d.delay) or nil, speed = has("speed") and tonumber(omsi.speed()) or 0,
    passengers = tonumber(total), seated = tonumber(seated), standing = tonumber(standing),
    room = room, height = size.height, width = 360, slim = room ~= nil and room < SLIM,
    fit = fit[tab], tab = tab, unread = messages.unseen(), wants = messages.wants(), at_work = signon.at_work(),
    show = M.show,
  }
end

-- --- the status line ------------------------------------------------------------------------------

-- how the duty stands, on its chip: the text and its colour
local function chip(c)
  local th, d = c.theme, c.duty
  if c.phase == "finished" then return tr("Duty finished"), th.dim end
  if c.phase == "break" then
    local left = ((c.trip or {}).departure or c.clock) - c.clock
    if left >= 0 then return tr("break, %{m} min left", { m = math.max(math.ceil(left / 60), 0) }), th.dim end
    return tr("%{m} min over", { m = math.max(math.ceil(-left / 60), 1) }), th.late
  end
  if c.phase == "before" then return tr("leaves in %{m} min", { m = math.max(math.ceil(((d.departure or c.clock) - c.clock) / 60), 1) }), th.dim end
  return P.delay(c.delay, th, tr)
end

-- the line badge, where the trip goes, trip 3/8 and the chip; without a duty the company
local function status_line(c)
  local th = c.theme
  if not c.duty then
    local name = assignment and (assignment.company or "") ~= "" and assignment.company or "Omsi-Hub"
    return { type = "row", gap = 8, children = {
      { type = "icon", name = "directions_bus", size = 18, color = th.dim },
      { type = "text", text = name, size = 14, weight = "bold", color = th.ink, grow = true, wrap = false },
    } }
  end
  local text, color = chip(c)
  return { type = "row", gap = 8, children = {
    P.plate(th, c.line, tr),
    { type = "text", text = c.terminus, size = 14, weight = "bold", color = th.ink, grow = true, wrap = false },
    { type = "text", text = string.format("%d/%d", c.k or 0, #c.trips), size = 12, weight = "medium", color = th.dim, wrap = false },
    { type = "badge", text = text, color = th.chip, text_color = color, weight = "bold" },
  } }
end

-- the tabs: each one's name, a count on it where it has one (not on the tab shown)
local function tabs(c, selected)
  local names = {}
  for k, t in ipairs(TABS) do
    local name = tr(t.tab or t.title)
    local b = t.badge and t.badge(c) or nil
    if b and b ~= 0 and b ~= "" and k ~= selected then
      -- (the name cut where the two do not fit: the count is what the driver looks for)
      local count = tostring(b)
      local most = TAB_LETTERS - 1 - #count
      local n = utf8.len(name)
      if n and n > most and most > 1 then name = name:sub(1, utf8.offset(name, most) - 1) .. "…" end
      name = name .. " " .. count
    end
    names[k] = name
  end
  return { type = "tabs", id = TABS_ID, tabs = names, selected = selected, color = c.theme.tab_on }
end

-- --- the phone, every second ----------------------------------------------------------------------

-- the phone's elements now
function M.children()
  ctx = context()
  local c = ctx
  -- (on the screen this second: the board is there; while it is away the phone is built still,
  -- to ask for it, but changes nothing)
  c.visible = docked and away == 0
  -- (a mode beginning brings its tab up: the break, an instruction to answer, the walk-around,
  -- a driver still to sign on - once, then the driver may go to another tab; one that began
  -- while the board was away, as it comes back)
  local modes = {}
  local switched = false
  for _, t in ipairs(TABS) do
    local m = t.mode and t.mode(c) or nil
    modes[t.id] = m
    if c.visible then
      if m and not moded[t.id] and not switched then
        M.show(t.id)
        switched = true
      end
      moded[t.id] = m ~= nil
    end
  end
  if c.visible and c.tab == "messages" and not c.slim then messages.seen() end
  c.fit = fit[c.tab]

  local children = { status_line(c) }
  if c.slim then
    local one = nil
    for _, t in ipairs(TABS) do
      if modes[t.id] and t.slim then one = one or t.slim(c) end
    end
    for _, id in ipairs(SLIM_ORDER) do
      local t = TABS[BY_ID[id]]
      if not one and t.slim then one = t.slim(c) end
    end
    if one then children[#children + 1] = one end
    return children
  end
  local k = BY_ID[c.tab]
  children[#children + 1] = tabs(c, k)
  local t = TABS[k]
  for _, e in ipairs(modes[t.id] or t.rows(c) or {}) do children[#children + 1] = e end
  return children
end

local function set()
  local panel = { anchor = "navigator", padding = PADDING, gap = GAP, children = M.children() }
  if version() >= 5 then panel.grab = true end
  layout.set(PANEL, panel)
end

-- the modules' own panels go (the phone has them) or come back
local function hand_over(on)
  messages.dock(on and not M.slim())
  strip.dock(on)
  ontime.dock(on)
  card.dock(on)
end

-- the entries the shown tab lists: as many as there is room for, found from how tall the panel
-- came out. An entry added that did not fit is not added again for less room than it took (a
-- line that wraps is taller than the margin).
local function fit_to(room, h)
  if room ~= size.room then need = {} end
  size = { room = room, height = h }
  local t = TABS[BY_ID[omsi.data.nav_tab]]
  if not (t.fit and h > 0 and room > 0) then return end
  local id = t.id
  if grew[id] and h > room then need[id] = h - grew[id] end
  grew[id] = nil
  if h > room and fit[id] > 1 then
    fit[id] = fit[id] - 1
  elseif h + math.max(26, need[id] or 0) < room and fit[id] < t.fit[2] then
    grew[id] = h
    fit[id] = fit[id] + 1
  end
end

-- whether the game's menu is open (it hides the panels: the board is not counted away then)
local function menu_open()
  return has("game.menu_open") and omsi.game.menu_open() == true
end

-- the phone now (`wanted`: a duty or an inbox to show). Whether it is in the navigator; when
-- not, the modules show their own panels.
function M.update(wanted)
  if not (wanted and docks()) then
    if shown then layout.remove(PANEL) end
    if docked then hand_over(false) end
    shown, docked, away = false, false, 0
    return false
  end
  local w, room, h, mine = dock()
  if w and mine then
    away = 0
    fit_to(room, h)
  elseif not menu_open() then
    -- (no board: the navigator is off, Shift+N put it away, another plugin has it or the city
    -- map is open; asked for still, so that it comes as soon as the board is there again)
    away = away + 1
  end
  local was = docked
  docked = away <= GRACE
  shown = true
  set()
  if docked ~= was then hand_over(docked) end
  -- (the room went to the slim phone or back: the inbox out of the phone or into it)
  if docked then messages.dock(not M.slim()) end
  return docked
end

-- the board back while the modules have their own panels, looked for every frame: the phone
-- takes them over at once, not up to a second later beside them. True when it did (main.lua's
-- own panel and the board on the screen go then).
function M.poll()
  if not shown or docked then return false end
  local w, room, h, mine = dock()
  if not (w and mine) then return false end
  away, docked = 0, true
  size = { room = room, height = h }
  hand_over(true)
  set()
  return true
end

-- the phone drawn again at once (a click on it)
function M.refresh()
  if shown then set() end
end

-- the panel of the module an element on the phone comes from: the inbox's, the strip's, the
-- company's panel (main.lua) for the rest
local function owner(e)
  if e:sub(1, 4) == "msg_" then return messages.PANEL end
  if e:sub(1, 6) == "strip_" then return strip.PANEL end
  return "omsihub"
end

-- a click on the phone: the tab module whose element it is, else the module it came from.
-- Returns the panel the click is for (nil: handled here).
function M.route(p, element)
  if p ~= PANEL then return p end
  local e = element or ""
  local c = ctx or context()
  local shown_tab = TABS[BY_ID[omsi.data.nav_tab]]
  if shown_tab.click and shown_tab.click(c, e) then return nil end
  for _, t in ipairs(TABS) do
    if t ~= shown_tab and t.click and t.click(c, e) then return nil end
  end
  return owner(e)
end

-- a change on the phone: a tab chosen, or what a tab module or another module takes
function M.route_change(p, element, value)
  if p ~= PANEL then return p end
  if element == TABS_ID then
    local t = TABS[tonumber(value) or 0]
    if t then M.show(t.id) end
    return nil
  end
  local c = ctx or context()
  for _, t in ipairs(TABS) do
    if t.change and t.change(c, element or "", value) then return nil end
  end
  return owner(element or "")
end

-- F9: the Messages tab shown, or the tab before it again
local before = "ride"
function M.key(key, down)
  if not (inbox_inside() and key == "F9" and down) then return false end
  if omsi.data.nav_tab == "messages" then
    M.show(before)
  else
    before = omsi.data.nav_tab
    M.show("messages")
  end
  M.refresh()
  return true
end

function M.stop()
  if shown then layout.remove(PANEL) end
  shown, docked = false, false
end

return M
