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
- `font-size` (`px`)
- `margin`, `padding`, `width`, `max-width` (`px`)
- `line-height` (unitless or `px`)
- `text-align` (`left`/`start`, `center`, `right`/`end`)

## Properties (next)

- `border`, `height`
- `font-weight`, `font-family` (engine fonts only)
- `list-style`, `white-space`
- `em`, `rem`, `%`

## Origins

1. User-agent (`frihart-style::ua_style`)
2. User (`user.css` in the profile)
3. Author (`<style>` in the document)

See [engine.md](engine.md).
