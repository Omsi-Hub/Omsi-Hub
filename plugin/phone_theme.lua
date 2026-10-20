-- Omsi-Hub's plugin: the phone's look (nav.lua and its tabs, phone_*.lua) - one set of inks for
-- the navigator's dark card and one for its light one, and the few ways the phone writes things.
--
-- Colour means one thing each: time only in late / on time / early (the navigator's own colours
-- and its line of ±59 s, so the phone and the map never disagree); the line's yellow badge for
-- what the trip is; yellow for the action to take, indigo for another one, grey to decline;
-- everything else neutral ink. The navigator draws its card under the phone (dark rgb(30,30,32)
-- or white): the phone's own panel stays see-through, and every text says its ink.

local M = {}

-- (within this many seconds of the timetable the bus is on time, as on the navigator's map)
M.ON_TIME = 59

local DARK = {
  light = false,
  ink = "#F2F2F7", dim = "#A1A1A8", faint = "#6E6E73",
  -- hairlines, a field's faint fill, a bar's track, the time chip's fill
  line = "#3A3A3C", field = "#FFFFFF14", track = "#FFFFFF1F", chip = "#FFFFFF1A",
  -- the line's badge and its ink
  badge = "#FFD23F", badge_ink = "#1A1400",
  -- time: late, on time, early
  late = "#FF453A", on_time = "#2FA84F", early = "#2F7CF6",
  -- buttons: the action to take (the game inks it dark), another one (light ink), decline
  primary = "#FFD23F", secondary = "#4A5BD6", neutral = "#48484A",
  -- the tab shown (the game inks it), the next stop's accent
  tab_on = "#F2F2F7", accent = "#FFD23F",
}

local LIGHT = {
  light = true,
  ink = "#1C1C1E", dim = "#6E6E73", faint = "#A1A1A8",
  line = "#D1D1D6", field = "#0000000D", track = "#0000001A", chip = "#0000000F",
  badge = "#FFD23F", badge_ink = "#1A1400",
  late = "#FF453A", on_time = "#2FA84F", early = "#2F7CF6",
  primary = "#FFD23F", secondary = "#4A5BD6", neutral = "#E5E5EA",
  tab_on = "#1C1C1E", accent = "#FFD23F",
}

-- the inks for the navigator's light card (`light`) or its dark one
function M.theme(light)
  return light and LIGHT or DARK
end

-- --- times ---------------------------------------------------------------------------------------

-- seconds of the day as 07:45
function M.hhmm(t)
  local m = math.floor((t or 0) / 60)
  return string.format("%02d:%02d", (m // 60) % 24, m % 60)
end

-- seconds as 2:05 (none below 0)
function M.mmss(s)
  s = math.max(math.floor((s or 0) + 0.5), 0)
  return string.format("%d:%02d", s // 60, s % 60)
end

-- how the bus stands: "late", "on_time" or "early" (seconds late; early below 0)
function M.punctuality(delay)
  delay = delay or 0
  if delay > M.ON_TIME then return "late" end
  if delay < -M.ON_TIME then return "early" end
  return "on_time"
end

-- the delay's colour in `theme`
function M.delay_color(delay, theme)
  return theme[M.punctuality(delay)]
end

-- the delay as the phone writes it: +2:14, −1:26 (a real minus) or "on time"
function M.delay_text(delay, tr)
  delay = delay or 0
  if M.punctuality(delay) == "on_time" then return (tr or tostring)("on time") end
  local s = math.floor(math.abs(delay) + 0.5)
  return string.format("%s%d:%02d", delay < 0 and "−" or "+", s // 60, s % 60)
end

-- both: the text and its colour
function M.delay(delay, theme, tr)
  return M.delay_text(delay, tr), M.delay_color(delay, theme)
end

-- --- texts ---------------------------------------------------------------------------------------

-- the trip's terminus as the driver reads it: without the line in front ("5 Rathaus") and
-- without the code the timetable pads on behind ("Bismarckplatz      307")
function M.terminus(line, terminus)
  local t = terminus or ""
  line = line or ""
  if line ~= "" and t:sub(1, #line + 1) == line .. " " then t = t:sub(#line + 2) end
  t = t:gsub("%s%s+%S+%s*$", "")
  if line ~= "" and #t > #line + 1 and t:sub(-#line - 1) == " " .. line then t = t:sub(1, -#line - 2) end
  return (t:gsub("^%s+", ""):gsub("%s+$", ""))
end

-- a text in capitals (the stats' labels), the accented letters too: Latin, Polish, Cyrillic
function M.caps(s)
  local out = {}
  for _, c in utf8.codes(s or "") do
    if c >= 0x61 and c <= 0x7A then c = c - 0x20
    elseif (c >= 0xE0 and c <= 0xFE and c ~= 0xF7) or (c >= 0x430 and c <= 0x44F) then c = c - 0x20
    elseif c >= 0x450 and c <= 0x45F then c = c - 0x50
    elseif c == 0x491 then c = 0x490
    elseif (c >= 0x100 and c <= 0x137) or (c >= 0x14A and c <= 0x177) then c = c - (c % 2)
    elseif (c >= 0x139 and c <= 0x148) or (c >= 0x179 and c <= 0x17E) then c = c - ((c + 1) % 2)
    end
    out[#out + 1] = utf8.char(c)
  end
  return table.concat(out)
end

-- --- elements ------------------------------------------------------------------------------------

-- the line's badge (an empty run: a faint tag saying so; `tr` the plugin's texts)
function M.plate(theme, line, tr)
  if (line or "") ~= "" then return { type = "badge", text = line, color = theme.badge, text_color = theme.badge_ink, weight = "bold" } end
  return { type = "badge", text = (tr or tostring)("Empty run"), color = theme.chip, text_color = theme.dim }
end

-- a text in the theme's ink (`ink`: "ink", "dim" or "faint", or a colour), size 13 unless said
function M.text(theme, text, ink, size, weight)
  return { type = "text", text = text or "", size = size or 13, weight = weight or "regular", color = theme[ink or "ink"] or ink }
end

-- a stat's small label: capitals, dim
function M.label(theme, text)
  return { type = "text", text = M.caps(text), size = 10.5, weight = "medium", color = theme.dim, wrap = false }
end

-- a stat: its big number over its label (`color`: the number's own, else the ink). A tile in
-- an openOMSI with boxes (`boxes`), the two texts on their own else.
function M.stat(theme, label, value, color, boxes)
  local children = {
    { type = "text", text = value or "", size = 22, weight = "bold", color = color or theme.ink, wrap = false },
    M.label(theme, label),
  }
  if boxes then return { type = "box", color = theme.faint, grow = true, gap = 0, children = children } end
  return { type = "row", grow = true, gap = 6, children = { children[1], children[2] } }
end

return M
