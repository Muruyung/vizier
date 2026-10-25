# Contract: Notifications UI

**Feature**: `029-webui-notifications` | **Plan**: [../plan.md](../plan.md) | **Spec**: [../spec.md](../spec.md)

The UI has one new surface (the notification panel) and a bell in every page header. It composes existing design primitives; the only design-system extensions are the bell glyph, an `IconButton` count/busy state, a `ListItem` action slot, and an `AgentStartingCard` downloads/width slot.

## 1. Header bell

At the right end of the page header on every screen: the shared `PageFrame` (and therefore `SettingsFrame`), Home's header, the chat header, and the phone top bar (AppShell's, or the chat's own). It is the last item in the actions row, after the page's own buttons.

```
┌───────────────────────────────────────────────┐
│ Tasks            [New task]  [🔔 3]           │
└───────────────────────────────────────────────┘
```

- It is an **`IconButton`** (`variant="ghost"`, `icon="bell"`) and opens the panel in place; it is not a route link.
- The count is the person's unread count for visible agents. The badge is hidden at zero and capped at `99+`; the filter tabs show the true count. The badge is `aria-hidden`; the button's accessible name is `Notifications`, plus the unread count in words and the busy state (e.g. `Notifications, 3 unread, setting up`).
- While any visible agent is starting or downloading, a **separate busy dot** shows, distinct from the count. It is not an unread notification and does not increment the count.
- On a phone the bell lives in the top bar (one bell per screen); the page header below `lg` does not repeat it.
- Clicking it opens the panel. The entry is visible even with no agents; it opens the empty state.

**Design change**: `IconButton` gains an optional count badge and a separate busy flag, and stories for count / many / count+busy. The sidebar and `NavItem` are unchanged.

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
- Closing returns to the same screen and restores focus to the bell it was opened from.
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

## 5. Notification card

One notification is a **card**: a bordered `rounded-lg` surface with the agent's sigil (`md`, 34px), its name, the time, the stored summary, and the mark-read control at the right, inside the card.

```
┌───────────────────────────────────────────────┐
│ ▦ Scribe                         14:02   [✓] │
│ Task “Morning digest” ended without ...       │
└───────────────────────────────────────────────┘
```

- **No unread gutter or dot.** The read toggle's **eye** is also the state: closed (`eye-closed`) means unread, open (`eye`) means read. It sits on the title line, immediately **left of the time**, and is always visible so unread rows are scannable. The card's border is the live emerald (`border-live`) when unread and the neutral `border-border` when read; the summary tone follows (`text-soft` unread, `text-muted` read; `failed`/`success` keep their tone), and a visually hidden `Unread` is in the row's accessible text. Read and unread rows keep the same shape.
- **Sigil**: the agent's identicon, `md` (34px).
- **Title**: the agent's name (the agent store, falling back to `agent_id`).
- **Time**: `feedTime` (`14:02`, `yesterday`, `Oct 21`), mono/faint, right-aligned on the title line.
- **Subtitle**: the stored summary, `type-ui-para`.
- **Read toggle**: an `IconButton` beside the time, a **sibling** of the row's text button (valid HTML; a click never navigates):
  - unread → `eye-closed`, accessible name `Mark as read`
  - read → `eye`, accessible name `Mark as unread`
  - always visible: it is the unread indicator as well as the action.
- **Card click**: navigates to `href` and marks the notification read. The text button holds the sigil, agent and summary; the eye and the time share the top-right meta column.
- Rows are stacked with an 8px gap; there are no divider lines.
- Long summaries wrap; the full text stays available to assistive technology.

**Design change**: none beyond the bell's `IconButton` count/busy; the card is built from `Sigil` and `IconButton` with the panel's own spacing. `ListItem`'s `action` slot remains the primitive for rows in other lists.

## 5a. Arrival toast

When the list is **closed** and the 15 s poll finds a new notification, the shell shows one **timed toast** (top-right, just under the page header, the design `Toast`):

- Single arrival: the agent's name as the title and the summary as the message; clicking it marks the notification read and opens its destination. Tone follows the notification (`failed` → error, `success` → success, otherwise info).
- A burst (several at once): one toast, `N new notifications`, whose action opens the panel.
- It dismisses itself: about 6 s, 8 s for a failure. The inbox still holds the row; the toast is a nudge, not the record.
- No toast while the panel is open — the row is already at the top of the list.
- A `chat_reply` for the conversation currently on screen is neither toasted nor counted: it is marked read immediately, so the bell does not move for a reply the person is already looking at. A reply for another conversation toasts normally.

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

- The header bell is a button with an accessible name including “Notifications”, the unread count and the busy state. The badge is not the only carrier.
- The panel is a modal dialog named `Notifications`; focus is trapped, Escape closes, focus returns to the bell.
- Filters are a named group (`Filter notifications`) with pressed state; counts are read with the labels.
- Unread is never colour-only: the row's accessible name says `Unread`, the eye is closed, the card is outlined in emerald and the summary tone changes.
- Failure is conveyed by wording and colour; ready uses wording; unread uses the summary's tone and the hidden word (no dot).
- The setup card is `role="status"`; the progress bar has `aria-valuemin/max/now` and a label, or is indeterminate.
- The mark action has an explicit label; its hit area is large enough on touch; it is keyboard reachable and visible on focus.
- Empty/error states are announced (`role="alert"` for errors) and the error has a Retry button. A toast is `role="status"` (polite) or `role="alert"` for a failure, and auto-dismisses.
- No horizontal scrolling at 390px; the panel and setup cards fit the viewport.

## 10. Test selectors and Playwright expectations

- Bell: role `button`, name `/Notifications/`, in the page header (desktop) or the top bar (phone). The count and busy state are in the accessible name.
- Panel: role `dialog`, name `Notifications`.
- Filters: group name `Filter notifications`; buttons `All`, `Unread`, `Read` with their counts (use a regex).
- Mark all: role `button`, name `Mark all read`.
- Row action: `Mark as read` / `Mark as unread`.
- Home: `getByText('Since you were away')` has count 0; the notification list contains the event text instead.
- Phone: the top bar's bell → dialog is full-screen (viewport-sized); close returns focus to the bell.
- e2e fixture: a unique dummyplug agent plus an MCP integration created through the API gives a deterministic `integration` notification with the test actor's name.

## 11. Component mapping

| Element | Source |
|---|---|
| Panel | `SlideOver` / `SlideOverPanel` |
| Header bell | `IconButton` (new count/busy) mounted by `PageFrame`/`SettingsFrame`, Home, the chat header and the phone top bar |
| Bell | new `bell` icon in `design/src/icons/map.ts` |
| Filters | `FilterTabs` |
| Notification card | `Sigil` (md), `IconButton` action, bordered card |
| Setup card | `AgentStartingCard` (new downloads/width), `DotLoader` |
| Empty/loading/error | `EmptyState`, `Skeleton` |
| Arrival toast | `Toast` (new `onSelect`), `ToastRegion` |
| Pagination | `LoadMore` |
| Actions | `IconButton`, `TextLink` |
| Chat line | small text + `DotLoader` + `TextLink` |

Nothing new is added to the app's navigation routes; the panel is opened from the shell and is not a URL. This is intentional: notifications are personal, and preserving the current screen is the point of the panel. No service worker, VAPID key or push subscription is added: browser/Web Push is a separate, deferred delivery feature (the inbox is its future source).
