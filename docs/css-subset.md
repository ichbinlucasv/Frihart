# CSS subset

Frihart implements a documented subset. Unimplemented properties are
ignored. They are never guessed into a broken layout.

## Selectors (now)

- type (`p`, `h1`)
- universal (`*`)
- comma lists (`h1, h2`)

## Selectors (next)

- class, id, descendant, child
- `:root`, `:link`, `:visited` (partitioned)

## Properties (now)

- `display` (`block`, `inline`, `none`)
- `color`, `background-color` (named + `#rgb` / `#rrggbb`)
- `font-size` (`px`)
- `margin`, `padding`, `width` (`px`)
- `text-align` (`left`/`start`, `center`, `right`/`end`)

## Properties (next)

- `border`, `height`, `max-width`
- `font-weight`, `font-family` (engine fonts only)
- `line-height`, `list-style`, `white-space`
- `em`, `rem`, `%`

## Origins

User-agent styles live in `frihart-style::ua_style`. Author CSS comes
from `<style>` tags and the extra sheet passed into the pipeline.
A profile `user.css` is not read yet.

See [engine.md](engine.md).
