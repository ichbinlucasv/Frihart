# Engine spine

One function turns HTML into something chrome can paint:

```
html bytes
  → frihart-html (tokenize + tree + fragments)
  → frihart-css  (author <style> + profile user.css)
  → frihart-style (UA, then user, then author)
  → frihart-layout (block flow, cosmic-text wrap)
  → frihart-gfx (display list, including link hits)
  → frihart-chrome (software paint)
```

`frihart-pipeline::layout_html` is that function. Chrome prefers
`frihart --content-worker` (sandboxed) and falls back in-process.

`frihart-dom` is an arena of `NodeId`s over the same tree. Chrome does
not mutate it.

## What is on

- Headings, paragraphs, lists, definition lists, pre/code, blockquote,
  `hr`, br, img boxes (alt only)
- CSS: `em`/`rem`/`%` font-size, `font-weight`, `border`, `height`
- Tables as a column grid, including `<caption>`
- Links: one display-list path, clickable
- Form fields on the display list (GET submit)
- Identity autofill (never passwords)
- rustls fetch, first-party cookies, HTTPS-only
- Tor tabs via SOCKS5 only (fail closed)
- Non-HTML responses saved to `~/Downloads` at 0600, never executed
- View-source as `Document::Source`
- Honest claim list: `about:sites`, [sites.md](sites.md)

## What is off

- JavaScript (`frihart-js` refuses; `javascript:` URLs refused)
- Image decode (`frihart-media` sniffs only; img is a box)
- Flex/grid, SVG
- WebExtensions execution (install/audit only)

## Isolation

Crate seams match the future process tree. `frihart-ipc` messages exist
in-process. Phase 6 splits them without rewriting product types.
