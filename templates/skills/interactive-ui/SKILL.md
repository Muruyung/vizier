---
name: interactive-ui
author: vizier
description: Use before calling ui_render_interactive_ui (or any time you draw an HTML page, dashboard, chart, table or form in the WebUI) so the page matches the WebUI theme and fits the reply.
keywords:
- ui
- html
- page
- dashboard
- chart
- table
- form
- interactive
- render_interactive_ui
version: 1
---

# Interactive pages in the WebUI

`ui_render_interactive_ui(title=…, html=…)` draws a page inside your reply. It is in the `ui`
category and only exists while the person is on the WebUI. This skill is how to make that page
look like part of the WebUI rather than a bare web page.

## 1. The page is already themed: write plain HTML first

Every page is drawn with a stylesheet that matches the WebUI's light or dark theme. Before you
write any CSS, use what is already there:

- **Styled elements**: `h1`–`h4`, `p`, `a`, `small`, `ul`/`ol`, `table`/`th`/`td`, `code`, `pre`,
  `kbd`, `blockquote`, `hr`, `button`, `input`, `select`, `textarea`, `label`, `fieldset`,
  `details`/`summary`, `progress`, `meter`.
- **Layout classes**: `.card` (surface box with border and 8px corners), `.grid` (responsive
  columns, 180px minimum), `.row` (wrapping flex row with gaps), `.stack` (column with gaps),
  `.scroll` (horizontal scroll, for wide tables).
- **Content classes**: `.stat` (`<b>value</b><span>label</span>`), `.badge`, plus `.accent`,
  `.ok`, `.warn` or `.err`, `.bar` (track, with an inner `<span style="width:40%">`),
  `button.primary`, `.muted`, `.faint`, `.num` (tabular figures, right-aligned).
- **Variables**: `--text --muted --faint` (ink), `--surface --border --tint` (surfaces and
  subtle fills), `--accent --accent-soft --accent-text`, status `--ok --warn --error` with
  `--ok-bg --warn-bg --error-bg`, charts `--series-1` … `--series-6`, and `--radius --shadow
  --font --mono`.

The kit has zero specificity, so any rule you write wins. Add CSS only for what the kit lacks
(a chart's geometry, a custom layout), and build it from the variables, never from colours you
pick yourself. Hard-coded colours are what make a page look foreign, and they break in the
other theme.

A complete page can be this small:

```html
<h3>Release status</h3>
<div class="grid">
  <div class="card stat"><b>12</b><span>Merged this week</span></div>
  <div class="card stat"><b>3</b><span>Waiting on review</span></div>
  <div class="card stat"><b>1</b><span>Failing CI</span></div>
</div>
<table>
  <thead><tr><th>PR</th><th>Owner</th><th>State</th></tr></thead>
  <tbody>
    <tr><td>#212 Usage export</td><td>ana</td><td><span class="badge ok">merged</span></td></tr>
    <tr><td>#215 Retry budget</td><td>li</td><td><span class="badge warn">review</span></td></tr>
  </tbody>
</table>
```

## 2. Design for where it is drawn

- **It sits in a chat bubble.** Often 500–700px wide, sometimes a phone. No fixed widths, no
  full-page hero sections, no 100vh layouts. Use `.grid` or `flex-wrap` so things reflow, and
  wrap wide tables in `.scroll`.
- **Leave the background transparent.** The reply already sits on the WebUI background; use
  `.card` for boxes. Never set `body` or `html` backgrounds.
- **One point per page.** Lead with the answer (a stat, a headline row), then the detail. Your
  reply text carries the explanation, so the page does not need paragraphs of prose.
- **Compact, not cramped.** The kit's spacing matches the WebUI; don't add large paddings or
  font sizes over 1.6em.
- **Both themes.** If you write any colour at all, use a variable. Check that text on any fill
  you made still reads in both light and dark.

## 3. Charts

There is no network, so no chart libraries: draw with inline SVG (or HTML bars with `.bar`).

- Series colours are `var(--series-1)` … `var(--series-6)`, **in that order**, assigned to
  each entity once so a filter never repaints them. Six is the limit; fold the rest into
  "Other".
- SVG text is black by default and disappears in dark mode: put `fill="currentColor"` on the
  `<svg>` or the `<text>`, and use `var(--muted)` for axis labels.
- Grid lines `stroke="var(--border)"`, thin marks (2px lines), no dual axes.
- For two or more series, show a legend and label the marks directly where there is room. Some
  series colours are low-contrast on the light theme, so a value should never be readable only
  through its colour.
- Use `viewBox` with `width="100%"` so the chart scales with the bubble. Position the points
  of a dense strip or timeline (50+ items) absolutely in percentages instead of with a
  `repeat(N, 1fr)` grid, which won't shrink below its content and overflows.
- Hovering a mark should show its value (a `<title>` inside the SVG element is enough).
- A number is often better as a `.stat` than as a chart with a single bar.

## 4. Making it interactive

The page's script gets `vizier`:

- `vizier.send(text, data?)` posts `text` as the person's message and starts a turn. Use it for
  a real choice the person makes (pick an option, submit a form). It only works after they
  interacted with the page, at most once a second. Make `text` read naturally in the chat ("Pick
  plan B"), and put the machine-readable part in `data`.
- `vizier.saveState(value)` / `vizier.state` / `vizier.onState(cb)` remember UI state (a
  selected tab, a checked list) without a turn. On load, redraw from `vizier.state` if it is not
  `null`, so a page reopened later looks the way the person left it.
- Use real `<button>`s and `<label>`s around inputs, so the page works with a keyboard and a
  screen reader.

## 5. Building big pages

- The code-mode sandbox refuses one long string literal: a multi-line `"""…"""` of a couple of
  KB (about 150 lines) can already fail with `SyntaxError: Source is too deeply nested`. Build a
  large page from chunks of about 2 KB, e.g. `html = "".join([head, style, body, script])`,
  rather than one huge literal.
- Prefix your own class names per component (`.timeline-tick`, not `.tick` or `.rc`). A short
  generic name can collide with another rule and blow up a layout in a way that's hard to trace.
- Keep the page under 256 KB. Images only as `data:` URLs, and only small ones.
- Call the tool once per reply, with the final page. A second call replaces the first.

## 6. Check before you send

- If the page has logic (state, calculations, a game), run that logic first: parse the script
  (`new Function(code)` under node, if you have a shell with node) and drive its functions with
  a stubbed `document`/`vizier`.
- If you have a shell with a headless Chromium-based browser, render the page and measure it:
  `--dump-dom` with an appended script that reports `document.documentElement.scrollWidth`
  against `clientWidth`, and any element whose `getBoundingClientRect().right` exceeds the
  width, at about 500px and 1000px. Test 500px, not 375px: headless window sizes clamp.
- If you have neither, re-read the HTML once for fixed widths, hard-coded colours and missing
  `fill="currentColor"`; those three cause most broken pages.

## 7. When it says `deferred`

The tool returns `{"status": "deferred"}` when no WebUI tab is open. The page appears the next
time the person opens the conversation, so tell them that instead of saying it is on screen.
