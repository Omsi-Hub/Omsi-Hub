-- Omsi-Hub's plugin: signing on, as Omsi-Hub's own game had it in the navigator
-- (crates/omsi-app/src/nav_signon.rs) - the driver signs on with a personnel number and a code,
-- signs the duty order, and only then has the duty and its codes in the IBIS. The number and the
-- code are the driver's pass, which Omsi-Hub keeps (`~/.openomsi/personnel.json`) and gives in
-- the assignment; the on-board computer (Omsi-Hub's window, through companion.lua) shows the
-- stage this module says and signs on with the companion's `sign_on` and `accept_order`.
--
-- The stages: "signon" (the number and the code asked), "order" (the duty order to accept),
-- "work" (signed on, the duty accepted). Without a pass in the assignment the driver is at work
-- at once, as with sign-on switched off in Omsi-Hub; so too when no on-board computer shows to
-- sign on at (main.lua skips the sign-on then). The on-board computer shows the driver the pass
-- (Omsi-Hub's server gives it to its own window only).

local M = {}

local pass = nil
local wrong = false
-- the driver got past the sign-on once on this drive (signed on, or skipped it)
local past = false

M.stage = "work"

function M.setup(assignment)
  local p = assignment and assignment.personnel
  if type(p) == "table" and (p.number or "") ~= "" and (p.code or "") ~= "" then
    pass = { number = tostring(p.number), code = tostring(p.code) }
    M.stage = "signon"
  end
end

-- signing on (the on-board computer, the phone: companion.lua): the number and the code at
-- once; whether the driver is past them (signed on, or no sign-on asked)
function M.try(number, code)
  if M.stage ~= "signon" or not pass then return true end
  if tostring(number or "") == pass.number and tostring(code or "") == pass.code then
    M.stage, wrong, past = "order", false, true
  else
    wrong = true
  end
  return M.stage ~= "signon"
end

-- the duty order accepted: `accepted` is called; whether there was one to accept
function M.accept(accepted)
  if M.stage ~= "order" then return false end
  M.stage = "work"
  accepted()
  return true
end

-- the driver signed off: back at the sign-on, where there is a pass to sign on with again;
-- whether there was
function M.sign_off()
  if not pass then return false end
  M.stage, wrong = "signon", false
  return true
end

-- the last number and code were not the driver's
function M.wrong()
  return wrong
end

-- the driver drives without the duty (the phone's "Drive without duty"): at work, no order
function M.skip()
  M.stage, wrong, past = "work", false, true
end

-- the driver is at the sign-on for the first time on this drive (not after signing off)
function M.waiting()
  return M.stage == "signon" and not past
end

-- the driver is at work (signed on, or no sign-on asked)
function M.at_work()
  return M.stage == "work"
end

function M.signed()
  return pass ~= nil
end

return M
