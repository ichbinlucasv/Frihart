# Sites we claim

Frihart claims **documents**, not “the web”. A named host lands here
only when a person has opened it in Frihart and it is readable without
JavaScript. The in-browser list is `about:sites`.

## Internal (always)

- `about:home`, `about:engine`, `about:campaigns`, `about:processes`,
  `about:linux`, `about:script`, `about:sites`

These are our chrome. They do not prove the HTML engine.

## Claimed (opened in this tree)

| URL | Why |
| --- | --- |
| https://example.com/ | Live HTML 2026-08-18. Title, `h1` at `1.5em` of parent, two paragraphs, IANA link via `a:link`. Body `60vw` + `margin: auto` centers the column. Light `#eee` canvas gets dark ink so it stays readable. `opacity` and `font-family` are ignored. |

## Target (open next)

Static or mostly-static HTML that fits the current subset (headings,
paragraphs, lists, `pre`, quotes, tables, `hr`, captions, links, img
boxes, GET forms):

| URL | Why |
| --- | --- |
| https://www.rfc-editor.org/rfc/rfc1918.html | Plain RFC document |
| https://www.gnu.org/philosophy/ | Essays |
| https://suckless.org/ | Small static pages |
| https://www.kernel.org/ | Simple landing + docs |

None of these is **claimed** until someone has read it in this tree
and the layout is honest.

## Will not claim

| URL | Why |
| --- | --- |
| Wikipedia | Infobox / more CSS first |
| GitHub, Gmail, maps, banks, social | JS apps |
| Proton Mail | JS app (Proton Pass is an external manager) |

A missing JS app is expected. A broken static layout is a bug.

See [engine.md](engine.md) and [css-subset.md](css-subset.md).
