# Engine spine

One function turns HTML into something chrome can paint:

```
html bytes
  → frihart-html (tokenize + tree)
  → frihart-css  (author <style> + extra sheet)
  → frihart-style (UA + author cascade)
  → frihart-layout (block flow)
  → frihart-gfx (display list)
  → frihart-chrome (software paint)
```

`frihart-pipeline::layout_html` is that function.

`frihart-dom` is an arena of `NodeId`s over the same tree. Chrome does
not mutate it.

## What is on

- Headings, paragraphs, links, form fields (GET submit)
- Identity autofill (never passwords)
- rustls fetch, first-party cookies, HTTPS-only
- View-source as `Document::Source`

## What is off

- JavaScript (`frihart-js` refuses)
- Image decode (`frihart-media` sniffs only)
- Flex/grid, tables, SVG
- Live Tor SOCKS (Tor tabs fail closed)
- WebExtensions execution (install/audit only)

## Isolation

Crate seams match the future process tree. `frihart-ipc` messages exist
in-process. Phase 6 splits them without rewriting product types.
