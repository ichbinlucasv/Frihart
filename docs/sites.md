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
| https://www.rfc-editor.org/rfc/rfc1918.html | Live HTML 2026-08-18. No `<title>` — title from `span.h1`. Nine `<pre>` pages + `<hr>` page breaks. Private nets `10/8`, `172.16/12`, `192.168/16` visible. Also checked at 5120×1440 (G9-class). |
| https://suckless.org/ | Live HTML 2026-08-18. Title, News, dwm/dmenu. Protocol-relative `//` links become https. No script. Also 5120×1440. |
| https://www.gnu.org/philosophy/ | Live HTML 2026-08-18. Title, “four essential freedoms”, essay list, `/philosophy/free-sw.html` links. `&mdash;` / `&ldquo;` decode. `@media` two-column CSS ignored (stacks). No script. Also 5120×1440. |
| https://www.kernel.org/ | Live HTML 2026-08-18. Title, About/Releases nav as clickable list links, releases table (`mainline` / `stable` / `7.2` cells). Download URLs in cells are text, not separate hits. External CSS not loaded (mobile header also shows). IE conditional scripts skipped. Also 5120×1440. |

## Target (open next)

The first five public claims are in. Next named host on the list:
`https://docs.kernel.org/` (not opened). Pick that or another static
document, open it, claim only if the layout is honest.

## Will not claim

| URL | Why |
| --- | --- |
| Wikipedia | Infobox / more CSS first |
| GitHub, Gmail, maps, banks, social | JS apps |
| Proton Mail | JS app (Proton Pass is an external manager) |

A missing JS app is expected. A broken static layout is a bug.

See [engine.md](engine.md) and [css-subset.md](css-subset.md).
