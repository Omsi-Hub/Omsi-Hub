-- Omsi-Hub's plugin: signing on in the phone (phone_duty.lua's mode) - a phone's keypad instead
-- of the company panel's text fields: the personnel number first, then the code (its digits as
-- dots), each with OK; a wrong pair says so and both are typed again. The keys are buttons, so
-- they take the mouse without F10 where the game gives the phone `grab` (ui version 5); the
-- checking is signon.lua's (`pad`, `press`), the duty order after it the company panel's.
--
-- The keys' ids are "duty_key_0" to "duty_key_9", "duty_key_back" and "duty_key_ok" (the Duty
-- tab's: its clicks come here).

local signon = require("signon")
local P = require("phone_theme")

local M = {}

-- (the keys, three to a row as on a phone)
local ROWS = { { "1", "2", "3" }, { "4", "5", "6" }, { "7", "8", "9" }, { "back", "0", "ok" } }

-- a key's button
local function key(ctx, k)
  local th, tr = ctx.theme, ctx.tr
  local b = { type = "button", id = "duty_key_" .. k, color = th.neutral, grow = true }
  if k == "back" then
    b.icon = "arrow_back"
  elseif k == "ok" then
    b.text, b.color = tr("OK"), th.primary
  else
    b.text = k
  end
  return b
end

-- what was typed: the number's digits, the code as dots (nothing yet: a faint dash)
local function typed(ctx, p)
  local th = ctx.theme
  local d = p.digits
  if d == "" then return { type = "text", text = "–", size = 24, weight = "bold", color = th.faint, align = "center" } end
  local shown = p.step == "code" and string.rep("•", #d) or d
  return { type = "text", text = shown, size = 24, weight = "bold", color = th.ink, align = "center", wrap = false }
end

-- the keypad's elements (nil: no sign-on asked now)
function M.rows(ctx)
  local p = signon.pad()
  if not p then return nil end
  local th, tr, a = ctx.theme, ctx.tr, ctx.assignment or {}
  local part = p.step == "code" and tr("Code") or tr("Personnel number")
  local out = {
    { type = "row", gap = 8, children = {
      { type = "icon", name = "badge", size = 18, color = th.accent },
      { type = "text", text = tr("Not signed on"), size = 14, weight = "bold", color = th.ink, grow = true, wrap = false },
    } },
    P.text(th, tr("Duty %{duty}: line %{line}, tour %{tour}", { duty = a.duty or "", line = a.line or "", tour = a.tour or "" }), "dim", 12),
    { type = "text", text = P.caps(part), size = 10.5, weight = "medium", color = th.dim, align = "center" },
    typed(ctx, p),
  }
  if p.wrong then out[#out + 1] = { type = "text", text = tr("Not known here. Your number and code are on your pass below."), size = 12, weight = "bold", color = th.ink, align = "center" } end
  for _, r in ipairs(ROWS) do
    out[#out + 1] = { type = "row", gap = 6, children = { key(ctx, r[1]), key(ctx, r[2]), key(ctx, r[3]) } }
  end
  local pass = signon.pass()
  if pass then
    out[#out + 1] = P.text(th, tr("Your pass") .. ": " .. tr("Personnel no. %{number} · code %{code}", { number = pass.number, code = pass.code }), "faint", 11.5)
  end
  if not ctx.can.grab then out[#out + 1] = P.text(th, tr("F10: use the panel with the mouse"), "faint", 11.5) end
  return out
end

-- a click on a key: whether it was one
function M.click(ctx, id)
  local k = id:match("^duty_key_(%w+)$")
  if not k then return false end
  signon.press(k)
  return true
end

return M
