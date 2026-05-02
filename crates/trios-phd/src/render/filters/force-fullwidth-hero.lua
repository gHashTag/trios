-- force-fullwidth-hero.lua
--
-- Replaces the FIRST image in a pandoc document with a hand-built
-- non-floating LaTeX figure that:
--   • is positioned [H] (immediately at the cursor, no floating)
--   • is centered
--   • spans width=\textwidth with keepaspectratio
--   • has the original alt-text as a small italic caption*
--
-- Also moves that figure to the very first block of the document, so the
-- hero is the first thing the reader sees regardless of whether the source
-- author put `# Title` before or after the `![alt](src)` line.

local function tex_escape(s)
  if not s then return "" end
  s = s:gsub("\\", "\\textbackslash{}")
  s = s:gsub("&",  "\\&")
  s = s:gsub("%%", "\\%%")
  s = s:gsub("%$", "\\$")
  s = s:gsub("#",  "\\#")
  s = s:gsub("_",  "\\_")
  s = s:gsub("{",  "\\{")
  s = s:gsub("}",  "\\}")
  s = s:gsub("~",  "\\textasciitilde{}")
  s = s:gsub("%^", "\\textasciicircum{}")
  return s
end

local function alt_text(image)
  -- caption is a list of inlines; flatten to plain text
  local out = {}
  for _, inl in ipairs(image.caption) do
    if inl.t == "Str"   then table.insert(out, inl.text)
    elseif inl.t == "Space" then table.insert(out, " ")
    elseif inl.t == "SoftBreak" or inl.t == "LineBreak" then table.insert(out, " ")
    elseif inl.t == "Code"  then table.insert(out, inl.text)
    end
  end
  return table.concat(out)
end

local function build_hero(image)
  local src     = image.src
  local caption = alt_text(image)
  -- The hero image is followed in body_md by an italic `*Figure — ChNN: ... *`
  -- expanded caption that we *keep* in the document (we only swallow the
  -- duplicate when the alt-text equals it). So the hero block itself does
  -- NOT need to render its own caption — doing so just risks Greek-glyph
  -- font fallback issues (φ → ¿) on some XeTeX configurations.
  local _ = caption  -- alt text intentionally unused here; see comment above
  local tex = string.format(
    "\\begin{figure}[H]\n" ..
    "  \\centering\n" ..
    "  \\includegraphics[width=\\textwidth,keepaspectratio]{%s}\n" ..
    "\\end{figure}\n",
    src
  )
  return pandoc.RawBlock("latex", tex)
end

-- Walk a list of inlines looking for an Image. Returns the first one
-- found, or nil.
local function find_image_in_inlines(inlines)
  for _, inl in ipairs(inlines) do
    if inl.t == "Image" then return inl end
  end
  return nil
end

function Pandoc(doc)
  local hero_block_idx = nil
  local hero_image     = nil

  for i, block in ipairs(doc.blocks) do
    -- Pandoc 3.x: `![alt](src)` on its own paragraph becomes a Figure block
    -- whose .content is a list of blocks (typically a Plain wrapping the Image).
    if block.t == "Figure" then
      for _, inner in ipairs(block.content) do
        if (inner.t == "Plain" or inner.t == "Para") then
          local img = find_image_in_inlines(inner.content)
          if img then
            hero_block_idx = i
            -- Figure blocks carry their own caption; prefer that for alt.
            -- Build a synthetic Image whose .caption is the Figure caption
            -- so build_hero gets the right text.
            local caption_inlines = {}
            if block.caption and block.caption.long then
              for _, b in ipairs(block.caption.long) do
                if (b.t == "Plain" or b.t == "Para") then
                  for _, inl in ipairs(b.content) do
                    table.insert(caption_inlines, inl)
                  end
                end
              end
            end
            if #caption_inlines == 0 then caption_inlines = img.caption end
            hero_image = pandoc.Image(caption_inlines, img.src, img.title or "", img.attr)
            break
          end
        end
      end
    elseif block.t == "Para" or block.t == "Plain" then
      local img = find_image_in_inlines(block.content)
      if img then
        hero_block_idx = i
        hero_image     = img
      end
    end
    if hero_image then break end
  end

  if hero_image == nil then return doc end

  local hero_block = build_hero(hero_image)

  table.remove(doc.blocks, hero_block_idx)

  -- After removing the original hero, the next block is often an italic
  -- expanded caption (e.g. `*Figure — Ch.1: ... (1200×800).*`). Detect
  -- and swallow it so we don't get a duplicate caption underneath the
  -- hero figure. Heuristic: a Para whose first inline is `Emph` and
  -- whose flattened text starts with "Figure —" or contains "Figure —".
  local function looks_like_caption(blk)
    if blk == nil then return false end
    if blk.t ~= "Para" and blk.t ~= "Plain" then return false end
    local first = blk.content[1]
    if not first or first.t ~= "Emph" then return false end
    local plain = pandoc.utils.stringify(blk)
    return plain:match("^%s*Figure%s*—") ~= nil
        or plain:match("^%s*Figure%s*\u{2014}") ~= nil
        or plain:match("^%s*Figure%s*%-") ~= nil
  end
  -- After table.remove, the (formerly second) block now sits at hero_block_idx.
  -- We deliberately KEEP the italic `*Figure — ChNN: ... *` paragraph: since
  -- the hero block above no longer carries its own caption, this paragraph
  -- becomes the (single) caption seen by the reader.
  -- if looks_like_caption(doc.blocks[hero_block_idx]) then
  --   table.remove(doc.blocks, hero_block_idx)
  -- end
  local _ = looks_like_caption  -- intentionally unused now; kept for future

  -- Place the hero image *immediately after* the chapter heading, not at
  -- the very top of the document. Walk the blocks looking for the first
  -- top-level Header (level 1 — i.e. `# Ch.NN — Title` from body_md, which
  -- pandoc converts to a \section* under our template). The hero is
  -- inserted right after that header. If no such header exists (some
  -- chapters skip it), fall back to the very first block.
  local insert_at = 1
  for i, b in ipairs(doc.blocks) do
    if b.t == "Header" and b.level == 1 then
      insert_at = i + 1
      break
    end
  end
  table.insert(doc.blocks, insert_at, hero_block)

  return doc
end
