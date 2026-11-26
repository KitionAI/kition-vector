# Type parity

> **Last reviewed:** 2026-11-17 · **Last updated:** 2026-11-26 · **Target:** Adobe Illustrator 2026 (30.x)

Type is Illustrator's largest single area by use (weight 13 of 105 across "Type core" and "Type advanced" in
[target-app-parity.md](target-app-parity.md)). This checklist lists the features one by one. Status is from the
code and tests (`crates/support/text`, `crates/engine/engine/src/cmd/type*.rs`, `textstyles.rs`, `crates/engine/tools/src/text.rs`)
against Illustrator's public documentation; **estimated** unless noted.

The in-page editor (`web/src/type/`, 2026-11-26, plan stage 5, feature `vector.editor.type`) now reaches these by hand:
point, area and path type on the canvas, the caret and selection keys, the Character and Paragraph panels with a font
menu, character attributes on selected characters, an input method's composition at the caret, and fonts loaded beyond
the engine's own (served families, a font file, a lazily loaded Chinese face).

## Summary

| Part | % | Remaining (h) |
|---|---:|---:|
| Type core (point, area, path type, Character/Paragraph, styles, threading, OpenType, fonts) | 80% | 14–19 |
| Type advanced (CJK composition, Middle Eastern, spelling, Touch Type, Retype, variable fonts) | 50% | 20–28 |

## Checklist

| Feature | Status | Notes |
|---|---|---|
| Point, area and type-on-a-path type; vertical variants | done / partial | vertical point, area and path type have initial support |
| Area Type Options (rows, columns, inset, first baseline, Auto Size, Shrink Text to Fit) | done | |
| Threaded text, Fit Headline, Text Wrap | done | |
| Type on a Path effects and options; brackets | done | |
| Character and Paragraph panels, styles with overrides | done | mixed-value display for differing paragraphs missing |
| OpenType panel, Glyphs panel, alternates | done | Highlight Alternate Glyphs preference unused |
| Font menu: samples, live preview, filters, favourites; font matching by any name; Find Font | done | |
| Variable fonts | partial | named instances as styles; axis sliders missing |
| Colour fonts (OpenType SVG / COLR) | partial | |
| Every-line and Single-line composers | done | tuned by eye, never compared side by side |
| Hyphenation… and Justification… options | missing | |
| Optical Margin Alignment | missing | |
| Tab stops and leaders; Tabs panel | partial | Tabs panel and stops with leaders in the model (`text.tabs.set`); leader drawing not verified against the docs |
| Hidden characters (Show Hidden Characters) | done | `type.hiddenCharacters` |
| Spell check (Check Spelling, Auto Spell Check, custom dictionary) | missing | needs an openly licensed dictionary |
| Smart Punctuation, Change Case, Find & Replace | done | |
| Touch Type tool | missing | listed in the toolbar, not implemented |
| Retype (identify fonts in outlines/images) | missing | Illustrator's uses a cloud model |
| Snap to Glyph | missing | |
| Middle Eastern: bidirectional layout, Paragraph Direction | done | |
| Middle Eastern: Character Direction, digit types, kashidas, Middle Eastern composers, split caret | missing | |
| CJK: input methods (marked text, candidate window) | done | |
| CJK: kinsoku (Hard, Soft, None), Line-end Punctuation Half Width, tate-chu-yoko (auto) | done / partial | custom kinsoku sets, the Mojikumi Settings dialog and the other mojikumi sets missing |
| CJK: ruby, proportional vertical metrics (`vpal`/`palt`), manual tate-chu-yoko | missing | |
| Inline symbols in text | beyond Illustrator | not scored |

## Revision history

| Date | Change | Summary |
|---|---|---|
| 2026-11-17 | major | First public version |
