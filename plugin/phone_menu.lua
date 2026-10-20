-- Omsi-Hub's plugin: lines of the company's duty in openOMSI's pause menu (Esc), where the
-- game has them (`omsi.ui.set_menu_item`, after 0.3.38): "Duty D-12 • trip 3/8" opens the
-- phone's Duty tab, "Message to control" its Messages tab with the quick reports open, "End the
-- duty" asks on the Duty tab whether to end it. Choosing a line closes the menu and the game
-- sends `menu_item` with its id (main.lua). The lines go when the duty has ended.
--
-- Without the phone (no navigator's board), and with the slim phone (no tabs to open): the duty
-- said on a card, the inbox's own panel opened with the mouse on it, and "End the duty" chosen
-- a second time within half a minute to end it.

local messages = require("messages")
local nav = require("nav")
local duty_tab = require("phone_duty")

local M = {}

-- the lines in their order in the menu, and their icons
local IDS = { "duty", "control", "end" }
local ICON = { duty = "badge", control = "forum", ["end"] = "flag" }
-- (a label has 40 characters at most; how long the second choice of "End the duty" may wait, s;
-- how many frames the mouse is asked for after the menu closed)
local LABEL_MAX = 40
local CONFIRM = 30
local FOCUS_TRIES = 120

local tr = function(t) return t end
local assignment = nil
local hooks = {}
-- the lines in the menu now (id -> label) and in the order they were put there; whether the
-- duty ran and has ended
local set = {}
local order = {}
local ran = false
local ended = false
-- (without the phone: "End the duty" chosen once, until then, s of the game; the frames left to
-- ask for the mouse in)
local armed = nil
local focus_wanted = 0

local function fn(name)
  local t = omsi
  for part in name:gmatch("[^%.]+") do
    if type(t) ~= "table" then return nil end
    t = t[part]
  end
  return type(t) == "function" and t or nil
end

-- `translate` the plugin's texts, `a` the assignment, `h` main.lua's: end_duty(), say(text,
-- title, icon, seconds)
function M.setup(translate, a, h)
  tr, assignment, hooks = translate, a, h or {}
end

local function duty()
  return fn("duty.get") and omsi.duty.get() or nil
end

local function now()
  local i = fn("info") and omsi.info() or nil
  return type(i) == "table" and tonumber(i.clock) or 0
end

-- a label the game takes: one line, 40 characters at most
local function fit(label)
  label = label:gsub("%s+", " ")
  if utf8.len(label) and utf8.len(label) > LABEL_MAX then label = label:sub(1, utf8.offset(label, LABEL_MAX) - 1) .. "…" end
  return label
end

-- the duty in a few words: "Duty D-12 • trip 3/8"
local function duty_label()
  local d = duty()
  local name = assignment and assignment.duty or ""
  if d then
    local trips = fn("duty.trips") and omsi.duty.trips() or {}
    return tr("Duty %{duty} • trip %{trip}/%{trips}", { duty = name, trip = d.trip or 0, trips = #trips })
  end
  return tr("Duty %{duty}", { duty = name })
end

-- the lines there should be now (id -> label)
local function wanted()
  local out = {}
  if not (assignment and (assignment.duty or "") ~= "") or ended then return out end
  local d = duty()
  out.duty = duty_label()
  if messages.has_inbox() then out.control = tr("Message to control") end
  if d and fn("duty.finish") then out["end"] = armed and tr("Choose again to end the duty") or tr("End the duty") end
  return out
end

-- the lines brought up to date (every second)
function M.update()
  if not fn("ui.set_menu_item") then return end
  local d = duty()
  if d then ran, ended = true, false elseif ran then ended = true end
  if armed and now() > armed then armed = nil end
  local want = wanted()
  local desired = {}
  for _, id in ipairs(IDS) do
    if want[id] then desired[#desired + 1] = id end
  end
  -- (the game keeps the lines in the order they were first put there: a line coming before
  -- others puts those after it in again)
  local keep = 0
  for i, id in ipairs(order) do
    if desired[i] ~= id then break end
    keep = i
  end
  for i = #order, keep + 1, -1 do
    if fn("ui.remove_menu_item") then omsi.ui.remove_menu_item(order[i]) end
    set[order[i]], order[i] = nil, nil
  end
  for i, id in ipairs(desired) do
    local label = fit(want[id])
    if set[id] ~= label and omsi.ui.set_menu_item(id, label, ICON[id]) then
      set[id] = label
      if i > #order then order[#order + 1] = id end
    end
  end
end

-- a line of the menu chosen (the game's `menu_item`)
function M.pick(id)
  if not set[id] then return end
  local phone = nav.active() and not nav.slim()
  if id == "duty" then
    if phone then
      nav.show("duty")
    elseif hooks.say then
      hooks.say(duty_label(), tr("Duty"), "badge", 8)
    end
  elseif id == "control" then
    if phone then
      nav.show("messages")
      messages.open_quick()
    elseif messages.open() and fn("ui.focus") then
      -- (its buttons take the mouse, as with F10: the game gives it only once its panels show
      -- again, a frame after the menu closed - asked again until then)
      if not omsi.ui.focus(true) then focus_wanted = FOCUS_TRIES end
    end
  elseif id == "end" then
    if phone then
      nav.show("duty")
      duty_tab.ask_end()
    elseif armed then
      armed = nil
      if hooks.end_duty then hooks.end_duty() end
    else
      armed = now() + CONFIRM
      if hooks.say then hooks.say(tr("Choose “End the duty” in the menu again to end it"), tr("Duty"), "flag", 8) end
    end
  end
  if phone then nav.refresh() end
  M.update()
end

-- every frame (main.lua): the mouse asked for the inbox opened from the menu, until it is given
function M.frame()
  if focus_wanted <= 0 then return end
  focus_wanted = focus_wanted - 1
  if not fn("ui.focus") or omsi.ui.focus(true) then focus_wanted = 0 end
end

return M
