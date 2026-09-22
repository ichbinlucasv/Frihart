# Frihart branding

**Canonical lockup:** [`frihart-lockup.png`](frihart-lockup.png) — gold
Templar-style cross + “Frihart” wordmark on black (user pick #2).

**App icon:** [`frihart-icon-1024.png`](frihart-icon-1024.png) /
[`frihart-icon-512.png`](frihart-icon-512.png) and sizes under
[`hicolor/`](hicolor/) — cross alone on black.

Colours: black `#0A0A0A`, gold `#FFD700` (chrome accents).

Do not regenerate logos unless asked. Older hearth explorations stay
under [`alts/`](alts/) for reference only; they are not shipping marks.

## Packaging

Linux packages install `hicolor/*/apps/frihart.png` and set
`Icon=frihart` in `packaging/linux/org.frihart.Frihart.desktop`.
Do not point the desktop entry at `frihart-lockup.png` (wordmark);
menus want the cross alone. See [docs/packaging.md](../docs/packaging.md).

