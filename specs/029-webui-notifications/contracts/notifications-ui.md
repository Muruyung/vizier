# Contract: Notifications UI

**Feature**: `029-webui-notifications` | **Plan**: [../plan.md](../plan.md) | **Spec**: [../spec.md](../spec.md)

The UI has one new surface (the notification panel) and one changed entry (the sidebar). It composes existing design primitives; the only design-system extensions are the bell glyph, a nav count/busy state, a `ListItem` action slot, and an `AgentStartingCard` downloads/width slot.

## 1. Sidebar entry

Placed immediately **above Settings** in the sidebar footer, below the page groups and the flexible spacer.

```
🔔 Notifications   3
⚙ Settings
◐ Theme
⇤ Collapse
@ you
```

- It is a **button** (`NavItem` with `onSelect`), not a route link, because the panel opens in place.
- The icon is the new `bell`. The label is `Notifications`.
- The count is the person's unread count for visible agents. Hidden when zero. Display is capped (`99+`); the filter tab shows the true count.
- While any visible agent is starting or downloading, a **separate busy indicator** shows (a small pulsing dot / the existing dot-wave language), distinct from the count. It is not an unread notification and does not increment the count.
- Expanded: a small mono count pill at the end of the row. Collapsed (64px rail): the bell, a dot for unread and/or busy, and a tooltip whose label includes the unread count (`Notifications · 3 unread`).
- The button's accessible name is `Notifications`, plus the unread count in words and the busy state; the visible badge is not the only carrier.
- Clicking closes the phone navigation drawer first (if open) and then opens the panel.
- The entry is visible even with no agents; it opens the empty state. It is never hidden by `hiddenPages`.

**Design change**: `NavItem` gains an optional unread count and a separate busy flag; `Sidebar` gains a `notifications` prop. Stories cover expanded/collapsed × unread/busy/both.

## 2. Panel shell

A right-hand `SlideOver`, 440px wide, full height, over the current screen. On a phone it is full-screen (`max-w-full`). The existing `SlideOver` supplies the native `<dialog>`, scrim, Escape-to-close, focus containment and the close button.

```
┌─ Notifications ─────────────────────────── × ┐
│ ┌ Setting up ──────────────────────────────┐ │
│ │ ●●●  Starting Scribe…           about 4m │ │
│ │      Pulling qwen3:32b                   │ │
│ │      [████████░░░░░░░░░] 42%             │ │
│ │      8.2 GB of 20.0 GB                   │ │
│ │      embedding · bge-small · done        │ │
│ └──────────────────────────────────────────┘ │
│ [All 26] [Unread 3] [Read 23]                │
│                            Mark all read     │
├──────────────────────────────────────────────┤
│ ● ▦ Scribe                        14:02  [✓] │
│   Task “Morning digest” ended without ...    │
├──────────────────────────────────────────────┤
│   ▦ Scribe                    yesterday      │
│   Dreamed: 4 journal entries.                │
├──────────────────────────────────────────────┤
│   ▦ Atlas                       Oct 21       │
│   Background batch reported back.            │
│                                              │
│                Load older                    │
└──────────────────────────────────────────────┘
```

- Header: `Notifications` (the dialog's accessible name) and the close control. Counts live in the filter tabs.
- Body: live setup section, then a **sticky** filter toolbar, then the scrolling list. The list keeps its position when a notification arrives.
- Opening the panel never marks anything read.
- Closing returns to the same screen and restores focus to the sidebar entry.
- Deselecting/clicking the scrim closes; Escape closes; the phone platform back should close it too (the implementation can push/pop a history entry or handle `popstate`).

**Mobile**: full-screen, own header, full-width setup cards, filters wrap to a second line when needed, “Mark all read” on its own right-aligned row when it does not fit, bottom safe-area padding, row actions always visible.

## 3. Live setup section

Shown only when at least one visible setup is `starting` or `downloading`.

- One `AgentStartingCard` per agent, stacked; multiple agents each get their own card.
- Card content: the 3-dot wave, `Starting {name}…`, the current step (`message`), a progress bar (determinate from the downloads' progress or indeterminate), percent, `completed of total` bytes when known, ETA when measurable, and a compact line per download (`kind · label · percent/status · error`).
- It uses `SetupState.name`; if missing, the agent store's name or the agent id.
- The card is `role="status"` and the bar is a `progressbar` with an accessible name/values; an unknown total is indeterminate, never a fake percent.
- No unread dot, no read action and no count. It disappears when the setup ends.
- A link/row action opens the agent's chat (or config for a failure); it does not mark anything read.
- The section scrolls with the panel; the filter toolbar below it sticks.
- It is the only detailed setup view. Chat shows a compact line, not this card.

**Design change**: `AgentStartingCard` gains a `downloads` slot and a width that fits both the chat column and the 400px panel content on a phone. Story with one and several downloads.

## 4. Filters and counts

- `FilterTabs` with `All`, `Unread`, `Read`; each shows the count from `NotificationCounts`.
- Default `All`. The selected filter is preserved across arriving notifications, read changes and panel close/reopen.
- Counts are global to the visible, retained set, not the current page.
- `Mark all read` is a `TextLink`/small action at the right of the toolbar; on a narrow screen it moves below the tabs. Disabled (or hidden) when unread is 0. It applies to **all** visible unread, including pages not loaded.
- No category filter, search or grouping in this release.

## 5. Notification row

Composed from `ListItem` with `shape="flush"`, `align="start"`, `action` (the new sibling slot), and `Sigil size="sm"`.

```
[gutter] [sigil]  Scribe                       14:02   [✓]
                  Task “Morning digest” ended without ...
```

- **Gutter**: a fixed 6–8px column; unread rows show a 6px ink dot, read rows leave it empty so nothing shifts. The dot is `aria-hidden`; the row's accessible text includes a visually hidden `Unread`.
- **Sigil**: the agent's identicon, `sm` (22px).
- **Title**: the agent's name (the agent store, falling back to `agent_id`).
- **Meta**: `feedTime` (`14:02`, `yesterday`, `Oct 21`), mono/faint, right-aligned.
- **Subtitle**: the stored summary, `type-ui-para`. Failure tone uses `text-failed-text`; success uses normal text (the wording says ready); default uses `text-muted`. Failure is carried by the wording and colour, never colour alone.
- **Action**: an `IconButton` sibling, outside the row's link/button:
  - unread → `check`, accessible name `Mark as read`
  - read → `rotate-ccw`, accessible name `Mark as unread`
  - desktop: hidden until row hover/focus-visible; phone: always visible; at least a 40×40 hit area.
  - Clicking it toggles read state and never navigates.
- **Row click**: navigates to `href` and marks read. Under the Unread filter the row leaves the list after the navigation; under All its dot disappears; under Read it would not have been shown until read.
- The row is a link or button with a name that includes the agent, the text and the time; the action is a separate control.
- Long titles/text truncate visually; the full text stays available to assistive technology (the subtitle is not `truncate`-only; it can wrap to two lines on a phone).

**Design change**: `ListItem` gains `action?: ReactNode` rendered as a sibling of the row's interactive element, so the mark button is never nested inside an anchor/button. Story with a row action.

## 6. States

| State | Rendering |
|---|---|
| First load | Three `Skeleton` line blocks; no false empty state. |
| All empty | `EmptyState kind="empty"` with the `bell` icon: “Nothing yet” / “Task runs, dreams, background jobs and integration changes will show up here.” |
| Unread empty | `EmptyState` with `circle-check`: “You’re all caught up.” / “New notifications will appear here.” |
| Read empty | `EmptyState` with `history`: “Nothing read yet.” / “Notifications you open or mark read will collect here.” |
| Load failed | `EmptyState kind="error"` with `alert`: “Couldn’t load notifications” + Retry. Keep the filter; keep the last known badge (do not show zero). |
| More pages | `LoadMore` “Load older” at the bottom; loading state; no duplicates. |
| New item while open | It appears at the top under a matching filter; the chosen filter is not reset and the list does not jump unexpectedly. Under Read it does not appear. |

## 7. Home and chat changes

### Home

- Remove the `AwayFeed` aside and the 340px right column entirely. Home's remaining content uses the width; no empty column and no duplicate feed.
- Mobile Home is unchanged (it never showed the aside).
- The “Since you were away” heading and empty-state text disappear from the product.

### Chat

- Replace the detailed `AgentStartingCard` with one compact line, e.g.:
  `Starting Scribe · Pulling qwen3:32b · 42% · Messages wait in line · View in Notifications`
  (`DotLoader`/small text; the link opens the panel).
- Keep the failed `Banner` with `Retry now` and `Open config` and the connection banners. The failure is also a notification; the banner is the contextual action where the person is blocked.
- The compact line disappears when the agent is ready; it is not a notification and does not carry a read state.

## 8. Polling and freshness

- `useNotifications.start()` (AppShell): `GET /notifications/count` every 15 s while the tab is visible; when the panel is open, refresh the current filter as well. New items merge at the top; the filter and scroll are preserved.
- The Mark-read/Unread and Mark-all responses carry counts; the store updates immediately and optimistically, then reconciles.
- Setup uses the existing `useAgents` poll. Change it to poll `/utils/setup-state` every 1 s while visible (not only when health already says starting), so the activity indicator appears within 2 s; progress and ETA update on the same poll.
- No WebSocket/SSE is added. Cross-tab read changes converge through the 15 s count/list poll (within the 30 s spec budget).
- On sign-out/app mount, reset the store so the previous person's counts cannot flash.

## 9. Accessibility

- The sidebar entry is a button with an accessible name including “Notifications”, the unread count and the busy state; the tooltip in the rail says the same. The badge is not the only carrier.
- The panel is a modal dialog named `Notifications`; focus is trapped, Escape closes, focus returns to the sidebar entry.
- Filters are a named group (`Filter notifications`) with pressed state; counts are read with the labels.
- The unread dot is decorative; the row's accessible name says `Unread`.
- Failure is conveyed by wording and colour; ready uses wording; unread uses the dot and the hidden word.
- The setup card is `role="status"`; the progress bar has `aria-valuemin/max/now` and a label, or is indeterminate.
- The mark action has an explicit label; its hit area is large enough on touch; it is keyboard reachable and visible on focus.
- Empty/error states are announced (`role="alert"` for errors) and the error has a Retry button.
- No horizontal scrolling at 390px; the panel and setup cards fit the viewport.

## 10. Test selectors and Playwright expectations

- Entry: role `button`, name `/Notifications/` (expanded and in the phone drawer). The count and busy state are in the accessible name.
- Panel: role `dialog`, name `Notifications`.
- Filters: group name `Filter notifications`; buttons `All`, `Unread`, `Read` with their counts (use a regex).
- Mark all: role `button`, name `Mark all read`.
- Row action: `Mark as read` / `Mark as unread`.
- Home: `getByText('Since you were away')` has count 0; the notification list contains the event text instead.
- Phone: open menu → button Notifications → dialog is full-screen (viewport-sized) and the drawer is gone; close returns focus to the entry.
- e2e fixture: a unique dummyplug agent plus an MCP integration created through the API gives a deterministic `integration` notification with the test actor's name.

## 11. Component mapping

| Element | Source |
|---|---|
| Panel | `SlideOver` / `SlideOverPanel` |
| Sidebar entry | `NavItem` (new count/busy) inside `Sidebar` (new prop) |
| Bell | new `bell` icon in `design/src/icons/map.ts` |
| Filters | `FilterTabs` |
| Row | `ListItem` (new `action` slot), `Sigil`, unread gutter |
| Setup card | `AgentStartingCard` (new downloads/width), `DotLoader` |
| Empty/loading/error | `EmptyState`, `Skeleton` |
| Pagination | `LoadMore` |
| Actions | `IconButton`, `TextLink` |
| Chat line | small text + `DotLoader` + `TextLink` |

Nothing new is added to the app's navigation routes; the panel is opened from the shell and is not a URL. This is intentional: notifications are personal, and preserving the current screen is the point of the panel. No service worker, VAPID key or push subscription is added: browser/Web Push is a separate, deferred delivery feature (the inbox is its future source).
