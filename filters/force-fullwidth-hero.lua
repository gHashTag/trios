--- force-fullwidth-hero.lua
---
--- Pandoc Lua filter for the PhD v5 chapter pipeline.
---
--- Behaviour:
---   1. Find the first standalone image in the document (a Para whose only
---      inline child is an Image).
---   2. Force its width attribute to "100%" and add the .hero-fullwidth class.
---   3. Move it to the very first block of the document, regardless of where
---      the author placed it in the Markdown.
---   4. Expose the resolved image src/caption as the template variables
---      `hero-image` and `hero-caption`, so chapter.template.tex can render
---      it via \chapterhero{}{} at the top.
---
--- Pairs with:
---   - templates/chapter.template.tex
---   - migrations/005_hero_fullwidth.sql (which prepends a Markdown image
---     with {width=100% .hero-fullwidth} attributes to every body_md).
---
--- The filter is idempotent: if a chapter has no standalone image, the
--- document is returned unchanged.

local function class_includes(classes, target)
  for _, c in ipairs(classes) do
    if c == target then return true end
  end
  return false
end

function Pandoc(doc)
  local hero_idx, hero_block, hero_image
  for i, block in ipairs(doc.blocks) do
    if block.t == "Para" and #block.content == 1 and block.content[1].t == "Image" then
      hero_idx, hero_block = i, block
      hero_image = block.content[1]
      break
    end
  end

  if not hero_image then return doc end

  -- Force full-width attributes.
  hero_image.attributes["width"] = "100%"
  if not class_includes(hero_image.classes, "hero-fullwidth") then
    hero_image.classes:insert("hero-fullwidth")
  end

  -- Move the hero block to position 1.
  if hero_idx ~= 1 then
    table.remove(doc.blocks, hero_idx)
    table.insert(doc.blocks, 1, hero_block)
  end

  -- Expose to the LaTeX template (chapter.template.tex).
  doc.meta["hero-image"]   = pandoc.MetaInlines({ pandoc.Str(hero_image.src) })
  local caption_text = pandoc.utils.stringify(hero_image.caption or {})
  if caption_text ~= "" then
    doc.meta["hero-caption"] = pandoc.MetaInlines({ pandoc.Str(caption_text) })
  end

  return doc
end
