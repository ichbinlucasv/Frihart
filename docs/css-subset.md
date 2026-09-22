# CSS subset

Frihart implements a documented subset. Unimplemented properties are
ignored. They are never guessed into a broken layout.

Honest coverage for Campaign D: **thirty-two** public static documents
are claimed (see [sites.md](sites.md) and `about:sites`). The first CSS
subset needed for that named list is **in**. External stylesheets are
still unused on most claims (nav stacks; that is honest, not a skip).
Flex, grid, and SVG stay off.

Site claims (live HTML → fixture → `sites.md`) are a separate queue
from leftover property work. Do not race the claim agent on the same
host files.

## Selectors (now)

- type (`p`, `h1`)
- class (`.lead`, `p.lead`)
- id (`#main`)
- descendant (`article p`)
- child (`nav > a`)
- universal (`*`)
- comma lists (`h1, h2`)
- `:link` and `:visited` (same color; no history leak)

`@media` / other at-rules are skipped. Nested rules inside them are
not applied. That is honest: we do not fake a two-column table layout.

## Selectors (later)

- `:root`
- attribute selectors

## Properties (now)

- `display` (`block`, `inline`, `none`)
- `color`, `background-color` (named + `#rgb` / `#rrggbb`)
- `font-size` (`px`, `em` of parent, `rem`, `%`)
- `font-weight` (`normal`/`bold`/`100`–`900`)
- `margin`, `padding`, `width`, `max-width`, `height` (`px`, `em`, `rem`, `vw`, `vh`, `%`)
- `margin: auto` centers the body column
- `border`, `border-width`, `border-color` (1px solid #333)
- `line-height` (unitless, `px`, `em`)
- `text-align` (`left`/`start`, `center`, `right`/`end`)
- `font-family` mapped to engine slots `sans` / `serif` / `mono` only.
  Unknown names (web fonts, emoji fonts) are skipped. No system inventory.
- `list-style` / `list-style-type` (`none`, `disc`, `circle`, `square`,
  `decimal`). Inherits from `ul`/`ol`. Position and image tokens ignored.
- `white-space` (`normal`, `nowrap`, `pre`, `pre-wrap`, `pre-line`).
  Soft-wrap off for `pre`/`nowrap`; on otherwise. `<pre>` UA is `pre`.

## Properties (next)

- (first subset complete for the named list)
- Leftover property families outside the claim queue may land as
  `D: CSS <property> leftovers (non-claim-queue)` — never by editing
  a host fixture the claim agent just touched.

UA extras: `hr` is a 2px rule fill; `caption` is centered 14px.
Letterboxing paint exists (pref off).

## Origins

1. User-agent (`frihart-style::ua_style`)
2. User (`user.css` in the profile)
3. Author (`<style>` in the document)

See [engine.md](engine.md) and [testing.md](testing.md).
