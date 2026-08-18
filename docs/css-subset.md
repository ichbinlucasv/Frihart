# CSS subset

Frihart implements a documented subset. Unimplemented properties are
ignored. They are never guessed into a broken layout.

## Selectors (now)

- type (`p`, `h1`)
- class (`.lead`, `p.lead`)
- id (`#main`)
- descendant (`article p`)
- child (`nav > a`)
- universal (`*`)
- comma lists (`h1, h2`)

## Selectors (later)

- `:root`, `:link`, `:visited` (partitioned)
- attribute selectors

## Properties (now)

- `display` (`block`, `inline`, `none`)
- `color`, `background-color` (named + `#rgb` / `#rrggbb`)
- `font-size` (`px`, `em`, `rem`, `%` of current size)
- `font-weight` (`normal`/`bold`/`100`–`900`)
- `margin`, `padding`, `width`, `max-width`, `height` (`px`, `em`, `rem`)
- `border`, `border-width`, `border-color` (1px solid #333)
- `line-height` (unitless, `px`, `em`)
- `text-align` (`left`/`start`, `center`, `right`/`end`)

## Properties (next)

- `font-family` (engine fonts only)
- `list-style`, `white-space`
- `%` width (needs a containing-block size)

UA extras: `hr` is a 2px rule fill; `caption` is centered 14px.

## Origins

1. User-agent (`frihart-style::ua_style`)
2. User (`user.css` in the profile)
3. Author (`<style>` in the document)

See [engine.md](engine.md).
