-- Omsi-Hub's plugin: the phone's Messages tab (nav.lua) - the control room's messages, newest
-- first, answered here, and the quick reports to control, in the phone's look (messages.lua
-- keeps the inbox, sends the answers and carries the instructions out; a new message shows its
-- count on the tab). How urgent a message is, its icon says, not a colour; an instruction has
-- Accept (yellow, the action) and Can't (grey). A message shown here is told read to Omsi-Hub.
-- Its mode: an instruction waiting for the driver's answer.

local messages = require("messages")
local P = require("phone_theme")

local M = {}

M.id = "messages"
M.title = "Messages"
-- (the tab itself: five fit side by side under the map, "Messages" does not)
M.tab = "Inbox"
M.icon = "forum"

-- the messages come in since the tab was looked at
function M.badge(ctx)
  return (ctx.unread or 0) > 0 and ctx.unread or nil
end

-- buttons two by two (a canned reply, a quick report): `prefix` and each choice's key their id
local function pairs_of(th, list, prefix, out)
  for i = 1, #list, 2 do
    local row = { type = "row", gap = 6, children = {} }
    for j = i, math.min(i + 1, #list) do
      local c = list[j]
      row.children[#row.children + 1] = { type = "button", id = prefix .. c.key, text = c.label or c.key, icon = (c.icon or "") ~= "" and c.icon or nil, color = th.neutral, grow = true }
    end
    out[#out + 1] = row
  end
end

-- the inbox's elements (`looked`: on the tab shown, its messages read)
local function inbox(ctx, looked)
  local th, tr = ctx.theme, ctx.tr
  local v = messages.view()
  local out = {}
  if v.hold then
    out[#out + 1] = { type = "badge", text = tr("Hold: %{time} left", { time = P.mmss(v.hold) }), color = th.primary, text_color = th.badge_ink, weight = "bold" }
  end
  if #v.messages == 0 then out[#out + 1] = P.text(th, tr("No messages"), "dim") end
  for k, m in ipairs(v.messages) do
    if #out > 0 then out[#out + 1] = { type = "divider" } end
    out[#out + 1] = { type = "row", gap = 6, children = {
      { type = "icon", name = m.icon, size = 16, color = th.ink },
      { type = "text", text = m.time .. "  ·  " .. m.from, size = 12, color = th.dim, grow = true, wrap = false },
      { type = "text", text = m.state, size = 12, color = th.dim, wrap = false },
    } }
    out[#out + 1] = P.text(th, m.text, "ink", 14, m.priority == "urgent" and "bold" or "regular")
    if m.reply ~= "" then out[#out + 1] = P.text(th, "↳ " .. m.reply, "dim", 12) end
    if m.outcome ~= "" then out[#out + 1] = P.text(th, m.outcome, "dim", 12) end
    if m.waiting then
      out[#out + 1] = { type = "row", gap = 6, children = {
        { type = "button", id = "msg_accept_" .. m.id, text = tr("Accept"), icon = "check", color = th.primary, grow = true },
        { type = "button", id = "msg_decline_" .. m.id, text = tr("Can't"), icon = "close", color = th.neutral, grow = true },
      } }
    elseif m.replies then
      pairs_of(th, v.replies, "msg_reply_" .. m.id .. "_", out)
    end
  end
  out[#out + 1] = { type = "divider" }
  out[#out + 1] = { type = "button", id = "msg_quick", text = v.quick_open and tr("Hide") or tr("Report to control"), icon = v.quick_open and "expand_less" or "campaign", color = th.secondary }
  if v.quick_open then pairs_of(th, v.quick, "msg_q_", out) end
  -- (looked at: read, for the control room)
  if looked then messages.shown() end
  return out
end

function M.rows(ctx)
  if not messages.has_inbox() then return { P.text(ctx.theme, ctx.tr("No messages"), "dim") } end
  return inbox(ctx, true)
end

-- (asked every second, whichever tab is shown)
function M.mode(ctx)
  if not ctx.wants then return nil end
  return inbox(ctx, ctx.tab == M.id)
end

-- the slim phone: a message to read or to answer, and the button that opens it
function M.slim(ctx)
  if not (ctx.wants or (ctx.unread or 0) > 0) then return nil end
  local th = ctx.theme
  return { type = "row", gap = 6, children = {
    { type = "icon", name = "forum", size = 16, color = th.ink },
    { type = "text", text = ctx.tr("Message from control"), size = 12.5, weight = "bold", color = th.ink, grow = true, wrap = false },
    { type = "button", id = "messages_open", icon = "forum", text = "", color = th.primary },
  } }
end

function M.click(ctx, id)
  if id ~= "messages_open" then return false end
  ctx.show("messages")
  return true
end

return M
