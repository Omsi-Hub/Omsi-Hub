-- Omsi-Hub's plugin: the phone's Messages tab (nav.lua) - the control room's messages, newest
-- first, answered here, and the quick reports to control (messages.lua draws them; a new one
-- shows its count on the tab). Its mode: an instruction waiting for the driver's answer.

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

function M.rows(ctx)
  if not messages.has_inbox() then return { P.text(ctx.theme, ctx.tr("No messages"), "dim") } end
  return messages.children()
end

function M.mode(ctx)
  if ctx.wants then return messages.children() end
  return nil
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
