# CSS subset

Frihart implements a documented subset. Unimplemented properties are
ignored. They are never guessed into a broken layout.

Honest coverage for Campaign D: **forty-one** public static documents
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
- `:root` (the `html` root element)
- attribute (2026-10-21): `[href]`, `[type="text"]`, `[rel~="nofollow"]`,
  `[lang|="en"]`, `[href^="https"]`, `[href$=".pdf"]`, `[href*="example"]`
  - names are ASCII case-insensitive (HTML). The first attribute of that
    name wins, as `getAttribute` does
  - values keep their case unless the `i` flag is set (`[type="text" i]`);
    `s` forces the sensitive default
  - an empty `^=` / `$=` / `*=` value, or a `~=` value that is empty or
    contains whitespace, never matches
  - a backslash in a value (hex escapes are not decoded) makes the
    selector invalid, so the rule is ignored
  - namespace prefixes (`*|href`, `xml:lang`) are not accepted; that
    selector is ignored

## At-rules (now)

`@media` blocks are evaluated against the viewport (2026-10-17;
`frihart_css::parse_stylesheet_media`, used by the pipeline for the user
and author sheets). Supported (Media Queries 4 subset):

- comma lists (any match), `not`, `only`
- media types `all`, `screen` (true), `print` (false)
- `width` / `height` with `min-` / `max-` prefixes and range syntax
  (`(width >= 40em)`, `(400px < width <= 900px)`); units `px`, `em`,
  `rem` (1em = 16px, the initial font size)
- `orientation: portrait | landscape`
- fixed, fingerprint-neutral answers, never read from the system:
  `prefers-color-scheme: light`, `prefers-reduced-motion` and
  `prefers-contrast: no-preference`, `hover: hover`, `pointer: fine`,
  `color` true, `monochrome` false, `scripting: none`
- nesting up to 4 levels; queries longer than 512 bytes never match

`@supports` blocks are evaluated too (2026-10-18;
`frihart_css::supports_matches`), so a page takes its fallback branch
when the own engine cannot render the modern one:

- `(prop: value)` is true only for declarations `frihart-style` applies;
  `display` answers true for `none`, `inline` and `block` only, so
  `(display: grid)` and `(display: flex)` are false today
- `selector(...)` is true when the selector grammar above accepts it
- `not`, `and`, `or`, nested parentheses; `and` mixed with `or` at one
  level is invalid and false, as are unknown functions such as
  `font-tech()`; conditions longer than 512 bytes are false

Unknown features evaluate to false (the block is skipped). Rules inside a
matching block go through the normal cascade, so properties we do not
implement are still ignored; we do not fake a two-column table layout.
Other at-rules (`@font-face`, `@keyframes`, `@import`, …) are skipped
with their nested rules.

## Selectors (later)

Sibling combinators (`+`, `~`) and pseudo-classes other than `:link`,
`:visited` and `:root` are not accepted; the selector is ignored.

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

## Custom properties (now)

- `--name: value` declarations, inherited root → element (an implicit
  `html` root is always first, so `:root { --x: … }` reaches the body).
  Names are case-sensitive.
- `var(--name)` and `var(--name, fallback)`, nested fallbacks included,
  in any property the subset already knows. A reference that cannot be
  resolved, a cycle, more than 8 levels or an expansion over 4 KiB makes
  the declaration ignored (never a guessed value).
- Not yet: `var()` inside inline `style=""` attributes, `@property`.

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
