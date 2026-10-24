-- Omsi-Hub's plugin: the phone's look (nav.lua and its tabs, phone_*.lua) - one set of inks for
-- the navigator's dark card and one for its light one, and the few ways the phone writes things.
--
-- Omsi-Hub's own look (ui/css/hub.css): its blue for what is chosen and for the action to take
-- (the tab shown, the next stop's card, the rail, the trip's progress, the buttons that go on),
-- its navy for the other buttons and the tiles, its inks for the texts. Colour means one thing
-- each besides: time only in late / on time / early (the navigator's own colours and its line of
-- ±59 s, so the phone and the map never disagree); the line's yellow plate for what the trip is.
-- The navigator draws its card under the phone (dark rgb(30,30,32) or white, its corners
-- rounded): the phone's own panel stays see-through - a background of its own would be square -
-- and every text, tile and button says its colour.

local M = {}

-- (within this many seconds of the timetable the bus is on time, as on the navigator's map)
M.ON_TIME = 59

local DARK = {
  light = false,
  ink = "#E8EBF2", dim = "#959DB0", faint = "#60687A",
  -- hairlines, a field's faint fill, a bar's track, a chip's fill
  line = "#343B4A", field = "#FFFFFF14", track = "#FFFFFF1F", chip = "#FFFFFF1A",
  -- the line's plate and its ink
  badge = "#FFD23F", badge_ink = "#1A1400",
  -- time: late, on time, early
  late = "#FF453A", on_time = "#2FA84F", early = "#2F7CF6",
  -- buttons: the action to take, another one, decline (each with light ink on it)
  primary = "#2A75F7", secondary = "#2A3A5C", neutral = "#2C3240",
  -- the tab shown and the others; the accent (the rail, the next stop, icons that point the
  -- way); the tiles' colour (a stat, a note: tinted by the game, a stripe of it on the left)
  tab_on = "#2A75F7", tab_off = "#2C3240", accent = "#4888F8", tile = "#5B7BB5",
}

local LIGHT = {
  light = true,
  ink = "#1E1C22", dim = "#645F62", faint = "#86807E",
  line = "#E5E0D9", field = "#0000000D", track = "#0000001A", chip = "#0000000F",
  badge = "#FFD23F", badge_ink = "#1A1400",
  late = "#FF453A", on_time = "#2FA84F", early = "#2F7CF6",
  -- (a light fill: the game inks it dark)
  primary = "#2A75F7", secondary = "#DCE6FB", neutral = "#EDE9E3",
  tab_on = "#2A75F7", tab_off = "#EDE9E3", accent = "#2A75F7", tile = "#2A75F7",
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

-- the line's plate (an empty run: a faint tag saying so; `tr` the plugin's texts). `wide`: as
-- wide as a plate of that many characters (a list's plates line up); `muted`: a trip done, its
-- plate grey.
function M.plate(theme, line, tr, wide, muted)
  if (line or "") == "" then return { type = "badge", text = (tr or tostring)("Empty run"), color = theme.chip, text_color = theme.dim } end
  local text = line
  local short = (wide or 0) - (utf8.len(line) or #line)
  -- (a space is about half a digit wide: two of them for each character missing, round it)
  if short > 0 then text = string.rep(" ", short) .. line .. string.rep(" ", short) end
  if muted then return { type = "badge", text = text, color = theme.chip, text_color = theme.dim, weight = "bold" } end
  return { type = "badge", text = text, color = theme.badge, text_color = theme.badge_ink, weight = "bold" }
end

-- a card added to `out`: `children` in a box tinted with `color` (the theme's tile unless
-- said), a stripe of it on the left; in an openOMSI without boxes (`boxes` false) the
-- elements on their own
function M.card(out, theme, boxes, children, color, gap)
  if not boxes then
    for _, e in ipairs(children) do out[#out + 1] = e end
    return
  end
  out[#out + 1] = { type = "box", color = color or theme.tile, gap = gap or 4, children = children }
end

-- a list's heading: a small dim label in capitals
function M.heading(theme, text)
  return { type = "text", text = M.caps(text), size = 10.5, weight = "bold", color = theme.dim, wrap = false }
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
-- an openOMSI with boxes (`boxes`): tinted with the number's colour where it has one (the
-- delay: late, on time, early), the theme's tile else; the two texts on their own without.
function M.stat(theme, label, value, color, boxes)
  local children = {
    { type = "text", text = value or "", size = 22, weight = "bold", color = color or theme.ink, wrap = false },
    M.label(theme, label),
  }
  if boxes then return { type = "box", color = color or theme.tile, grow = true, gap = 0, children = children } end
  return { type = "row", grow = true, gap = 6, children = { children[1], children[2] } }
end

-- a checkbox (`e`: its element) in the theme: its box in Omsi-Hub's blue, never the game's
-- amber; on the light card its label a text of its own beside it (the game writes a checkbox's
-- label in its light ink whatever its colour)
function M.check(theme, e)
  local box = {}
  for k, v in pairs(e) do box[k] = v end
  box.color = theme.primary
  if not theme.light or (e.text or "") == "" then return box end
  box.text, box.visible = "", nil
  return { type = "row", gap = 8, visible = e.visible, children = {
    box, { type = "text", text = e.text, size = 14, color = theme.ink, grow = true },
  } }
end

-- (the colours of the other modules' panels, as the phone writes them: their grey and their
-- orange dim, their red ink - red is the time's only)
local ADOPT = { ["#9AA3AD"] = "dim", ["#F58620"] = "dim", ["#F47F30"] = "dim", ["#FF5F57"] = "ink" }
-- (the buttons of theirs that are the action to take)
local PRIMARY = { signon = true, accept = true, start = true, strip_note_send = true, strip_chk_ok = true }

-- another module's elements (the company's panel, the sign-on, the strip's note and
-- walk-around) in the phone's look: copies, their texts, icons and tables in the theme's inks,
-- their buttons in its fills, their boxes and bars in its tile and accent, their checkboxes as
-- `check` draws them
function M.adopt(theme, list)
  local out = {}
  for _, e in ipairs(list or {}) do
    if type(e) == "table" then
      local c = {}
      for k, v in pairs(e) do c[k] = v end
      local t = c.type
      if t == "checkbox" then
        c = M.check(theme, c)
      elseif t == "text" or t == "icon" or t == "table" then
        if c.color == nil then
          c.color = theme.ink
        elseif ADOPT[c.color] then
          if t == "text" and c.color == "#FF5F57" then c.weight = "bold" end
          c.color = theme[ADOPT[c.color]]
        end
      elseif t == "button" and c.color == nil then
        c.color = PRIMARY[c.id or ""] and theme.primary or theme.neutral
      elseif (t == "box" or t == "bar") and (c.color == nil or ADOPT[c.color]) then
        -- (the game's amber, their orange: Omsi-Hub's tile, its blue for a bar)
        c.color = t == "box" and theme.tile or theme.accent
      end
      if t ~= "checkbox" and type(c.children) == "table" then c.children = M.adopt(theme, c.children) end
      out[#out + 1] = c
    end
  end
  return out
end

return M
