# Feature Specification: WebUI Notifications

**Feature Branch**: `029-webui-notifications`

**Created**: 2026-10-25

**Status**: Draft

**Input**: User description: "let's make notification system on webui, add notification icon the the sidebar above the setting icon perhaps, to list notifications, user could filter the read/unread notifications

this also replace the "Since you were away" section on home"

## Context

Today the only cross-agent account of what happened while a person was away is Home's “Since you were away” panel. It lists notable events — task runs that ended, dream cycles, background jobs, integration changes, batched channel replies and agents that cannot start — newest first, 20 at a time, each linking to the page where the person can see more.

That panel has three limits. It lives only on Home and only on wide screens, so a person on a chat, task or settings screen cannot tell that something happened. Its window is the person's previous visit, so events that happened during a long session, or before the previous visit, are invisible. And it has no memory of what the person has already looked at, so it cannot answer “is there anything I have not seen yet?”

This feature turns those notable events into a per-person **notification inbox**: a Notifications entry in the app navigation, immediately above Settings, carrying an unread badge and opening a list that can be filtered to all, unread or read notifications. The “Since you were away” panel and its previous-visit boundary are replaced; the same notable events remain available from the notifications entry on every screen.

Read state belongs to each person. Two people who can see the same agent share the notification itself but not its read state. Notifications are in-app only: no email, chat-platform message, desktop notification or sound. Per-category preferences, muting, search and grouping are out of scope for this release.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See what happened and open it from any screen (Priority: P1)

A person is working on an agent's tasks page when a scheduled task fails and a background job finishes. Without leaving the page, they notice an unread count beside Notifications in the navigation, open the notification list, and see both events with the agent's name, what happened and when. Selecting the failed task takes them to that task's run; selecting the background job takes them to the conversation where its report was delivered. Both are then marked read.

**Why this priority**: This is the feature. A notification system that cannot be reached from where the person already is, or that does not take them to the event, delivers no value.

**Independent Test**: With an agent that has produced at least one notable event, sign in, open a screen other than Home, confirm the Notifications entry and badge, open the list, select an event, and confirm the destination and the read change. Repeat with an empty inbox and with a failing agent.

**Acceptance Scenarios**:

1. **Given** a signed-in person and the app navigation, **When** any screen is shown, **Then** Notifications appears immediately above Settings with an icon, a readable label and the person's unread count for agents they may see.
2. **Given** the person has no unread notifications or no agent they may see, **When** the navigation renders, **Then** no unread badge is shown (not a zero).
3. **Given** the person has unread notifications, **When** they select Notifications, **Then** a list opens without leaving the current screen, newest first, and each row shows the agent, what happened, when it happened and whether it failed.
4. **Given** more notifications than fit one page, **When** the person chooses to load older ones, **Then** older notifications are appended in order with no duplicates and no gaps.
5. **Given** the navigation is collapsed to icons, **When** the person views the rail, **Then** Notifications is still present with an accessible name, a visible unread indicator and the same behaviour as the expanded entry.
6. **Given** a notification row, **When** the person selects it, **Then** its destination opens and that notification becomes read.
7. **Given** the notification list is open, **When** the person closes it, **Then** they are back where they were, with keyboard focus returned to the Notifications entry.

---

### User Story 2 - Separate read from unread and clear the inbox (Priority: P1)

A person returns after a week. The badge says 14. They open the list, read the failures first, mark a noisy channel summary as read without opening it, then use “Mark all read” to clear the rest. The badge disappears. Later they want to find something they had already dealt with, so they filter to Read and find it. The state is the same when they open vizier in another browser.

**Why this priority**: The request explicitly asks for read/unread filtering. Without a durable per-person read state, the list is only another version of the old feed.

**Independent Test**: Produce several notifications, open the list, mark one read, mark one unread, mark all read, reload, open a second tab, and verify the filters and badge at each step. Open the list without selecting anything and confirm nothing becomes read.

**Acceptance Scenarios**:

1. **Given** the notification list, **When** it renders, **Then** All, Unread and Read filters are offered with counts, the default is All, and every notification belongs to exactly one of Unread or Read.
2. **Given** unread notifications, **When** the person opens the list but selects nothing, **Then** every notification stays unread and the badge is unchanged.
3. **Given** a notification, **When** the person marks it read or unread, **Then** its state changes immediately, the filter counts and badge update, and the choice persists across reload, a new tab and another device.
4. **Given** unread notifications, **When** the person chooses “Mark all read”, **Then** every unread notification they may see becomes read in one action and the badge disappears, including notifications not yet loaded on the page.
5. **Given** the Unread filter, **When** the person marks the last unread notification read, **Then** the list shows the unread empty state and the badge disappears.
6. **Given** the Read filter, **When** a new notification arrives, **Then** it does not appear under Read and the person's chosen filter is not reset.
7. **Given** an empty filter, **When** it is selected, **Then** a specific empty state explains that there are no notifications in that state.

---

### User Story 3 - Everything the old panel showed is still accounted for (Priority: P1)

A person used to rely on Home's “Since you were away” panel. After this change, Home no longer has that panel, but every category it covered — an agent that could not start, a task run that ended, a dream cycle, a finished background job, an integration change and batched channel replies — appears in Notifications, with the same wording and the same destination. Nothing that used to be visible becomes invisible.

**Why this priority**: Removing the Home panel without covering its events would be a regression. The replacement must be at least as complete as what it replaces.

**Independent Test**: Trigger each of the six categories on a test agent, then confirm one notification per completed event in the list, with the expected text, failure emphasis and destination. Leave a task and a background job running and confirm neither is notified yet. Open Home and confirm the old panel is gone.

**Acceptance Scenarios**:

1. **Given** an agent that cannot start, **When** it enters the failed state, **Then** one notification describes the failure and links to the agent's configuration; repeated retries of the same failure do not add duplicates.
2. **Given** a scheduled task run, **When** it ends answered, without a response or interrupted, **Then** one notification names the task and outcome and links to that run, with non-answered outcomes marked as failures.
3. **Given** a dream cycle, **When** it completes, **Then** one notification states how many journal entries were written and links to the cycle.
4. **Given** a background job, **When** it is reported, cancelled, lost or undeliverable, **Then** one notification names the job kind and outcome and links to the agent's conversation; a job that is still running is not notified.
5. **Given** an integration, **When** it is created, changed, enabled, disabled, deleted, unlinked or its self-management grant changes, **Then** one notification names who acted and what changed and links to the matching integrations page.
6. **Given** replies in a Discord or Telegram conversation, **When** a new hourly batch is complete, **Then** one notification states how many replies were sent and links to that conversation.
7. **Given** Home at any viewport, **When** it renders, **Then** no “Since you were away” panel is present, and Home's remaining content uses the freed width rather than leaving an empty column.
8. **Given** any notification is produced or read, **When** it is generated or displayed, **Then** no agent turn or model call is started to do so.

---

### User Story 4 - See only what belongs to you and what you may see (Priority: P2)

A person who manages only one agent does not see notifications for agents they do not own. When their access to an agent is revoked, that agent's notifications disappear from their list and badge on the next refresh. When an agent is deleted, its notifications disappear for everyone. Another person's read choices never affect theirs.

**Why this priority**: The old feed already respected agent visibility. A cross-agent inbox that leaks other agents' activity, or that keeps orphaned notifications, is a security and correctness problem.

**Independent Test**: With two people having different agent access, confirm each sees only their agents' notifications and their own read state. Revoke access while the list is open, delete an agent, and confirm disappearance and badge updates. Attempt to reach another person's notification list through a direct route.

**Acceptance Scenarios**:

1. **Given** a person and their visible agents, **When** the list or badge is read, **Then** only notifications for those agents are included.
2. **Given** a person's access to an agent, **When** it is revoked, **Then** that agent's notifications leave the list and the unread count on the next refresh.
3. **Given** an agent, **When** it is deleted, **Then** none of its notifications or read states remain for any person.
4. **Given** two people who can both see the same agent, **When** one marks a notification read, **Then** the other's state is unchanged.
5. **Given** no valid session, **When** any notification read is attempted, **Then** it is refused and no notification content is returned; a signed-in person's list never contains another person's agents.
6. **Given** notifications older than the retention period, **When** the list is read, **Then** they are not included and the unread count does not include them.

---

### User Story 5 - Stay current while the app is open (Priority: P2)

A person leaves vizier open on a chat. A background job finishes. Within half a minute the badge rises without a reload; when they open the list, the new item is at the top. In a second tab, they mark it read; the first tab's badge and list update without a reload.

**Why this priority**: A notification that is only discovered on the next manual reload is barely better than the old panel. Live freshness and cross-tab consistency are what make the inbox trustworthy.

**Independent Test**: Keep the app open with the list closed and then open, trigger a notification, and measure the badge and list update. Mark it read in another tab and confirm both surfaces converge. Simulate a failed load and confirm the error and retry path.

**Acceptance Scenarios**:

1. **Given** the app is open, **When** a new notification is created, **Then** the badge updates within 30 seconds without a manual reload.
2. **Given** the list is open and the current filter matches the new notification's state, **When** it arrives, **Then** it appears at the top without resetting the chosen filter or moving the person's scroll position unexpectedly.
3. **Given** two tabs or windows are open, **When** one changes a notification's read state or marks all read, **Then** the other reflects the change within 30 seconds without a reload.
4. **Given** the list cannot be loaded, **When** the person opens it, **Then** an error state with a retry action is shown, the previous filter is kept, and the badge does not falsely read zero if the last successful load was not zero.
5. **Given** notifications with the same timestamp, **When** they are listed, **Then** their order is stable across requests and paging.

---

### User Story 6 - Use it on a phone and with assistive technology (Priority: P3)

A person on a phone opens the navigation drawer, finds Notifications above Settings, and gets the same list full-screen. A keyboard-only person can reach the entry, open the list, move through rows, read them with a screen reader, mark read/unread and close with Escape. Failure is not conveyed by colour alone.

**Why this priority**: The feature must be usable everywhere the old Home panel was not, but basic desktop use is the primary path.

**Independent Test**: Complete the main journeys at a phone viewport and with keyboard only plus a screen reader, checking labels, focus order and focus return. Check failure rows do not rely on colour alone.

**Acceptance Scenarios**:

1. **Given** a phone viewport, **When** the navigation drawer is opened, **Then** Notifications is present above Settings with its unread indicator, and selecting it opens a full-screen list with the same content and filters.
2. **Given** keyboard-only use, **When** the person tabs to Notifications, opens it, moves through rows and closes it, **Then** every action is operable, focus is contained while open, Escape closes, and focus returns to the entry.
3. **Given** a screen reader, **When** the entry, badge, filters, rows and actions are read, **Then** each has a meaningful name and the unread count is announced, without relying on the visual badge alone.
4. **Given** a failed notification, **When** it is shown, **Then** its failure is conveyed by text and/or icon in addition to any colour.

---

### Edge Cases

- **No notifications at all**: the list shows an empty state that explains what will appear there, and no badge is shown.
- **Unread count above the display cap**: the badge shows a capped value (for example “99+”) while the Unread filter count remains accurate.
- **More unread notifications than one page**: “Mark all read” clears every unread notification the person may see, not only the loaded page.
- **New notification while filtered to Read**: it does not appear in the current list, the filter is preserved, and the badge still updates.
- **A notification arrives for an agent the person just lost access to**: it is not shown, counted or announced.
- **Notification destination already gone**: for example a task deleted after its run notified. The notification remains readable in history; selecting it opens the closest surviving page or a clear not-found state rather than a dead end.
- **Agent deleted**: its notifications and every person's read state for them disappear, and counts decrease immediately on refresh.
- **Two tabs act at once**: marking the same notification read twice is harmless; “Mark all read” in one tab and a new notification in the other converges without making the new one read.
- **Marking unread**: a read notification can be returned to Unread so the Unread filter is not one-way.
- **Same timestamp**: notifications created in the same instant keep a stable order across pages.
- **Very long text**: rows stay readable without breaking the layout, and the full text remains available to assistive technology.
- **Time zone or clock change**: times remain ordered by when the event occurred and are displayed in the person's current local time.
- **First availability of the feature**: existing historical events are not presented as a pile of unread notifications; the badge starts at zero.
- **No visible agents**: the list is empty and no events from outside the person's visibility are inferred or shown.
- **Failed load or offline**: the person sees an error with retry; the interface never silently presents an empty inbox as if nothing happened.
- **Own action**: a change the person made themselves still appears as a notification, matching the old panel's coverage.

## Requirements *(mandatory)*

### Functional Requirements

**Entry point and presentation**

- **FR-001**: The app navigation MUST offer a Notifications entry immediately above Settings on every screen where the navigation is shown, with an icon, a readable label and an accessible name.
- **FR-002**: The entry MUST show the signed-in person's unread count for notifications they may see, hide the badge when the count is zero, and cap the displayed value while keeping the true count available in the list.
- **FR-003**: Selecting the entry MUST open the notification list without navigating away from the current screen; closing it MUST return the person to that screen with focus restored to the entry.
- **FR-004**: The list MUST show notifications newest first, in pages, with a way to load older ones and no duplicates or gaps across pages.
- **FR-005**: Each notification MUST show which agent it concerns, what happened, when it happened, and whether it failed, and MUST lead to the place where the person can see more.
- **FR-006**: The entry and its badge MUST remain usable when the navigation is collapsed to an icon rail.
- **FR-007**: On a phone viewport the entry MUST be available in the navigation drawer and MUST open the same list full-screen.

**What becomes a notification**

- **FR-008**: Notifications MUST cover the notable events the removed Home panel covered: an agent that cannot start, a completed task run, a completed dream cycle, a finished background job, an integration change and a batch of channel replies.
- **FR-009**: Only completed or terminal outcomes MUST be notified. Work that is still running MUST NOT appear, and a single failing episode MUST NOT produce a new notification for every retry.
- **FR-010**: A task run MUST be notified when it ends answered, without a response or interrupted; a non-answered outcome MUST be marked as a failure.
- **FR-011**: A background job MUST be notified when it is reported, cancelled, lost or undeliverable, naming the job kind and outcome.
- **FR-012**: An integration change MUST name who acted and what changed, for creation, editing, enabling, disabling, deletion, unlinking and self-management grants.
- **FR-013**: Channel replies MUST be summarised per conversation and hour, stating how many replies were sent.
- **FR-014**: An agent that cannot start MUST produce one notification when it enters the failed state, linking to its configuration. Retries within the same failure episode MUST NOT add duplicates; after the agent recovers and later fails again, a new episode MUST notify.
- **FR-015**: Notification content MUST be derived from recorded facts alone. Producing, listing or opening a notification MUST NOT start an agent turn or a model call.
- **FR-016**: Every notification MUST have a stable identity and a destination, so paging, read state and cross-tab updates agree and a notification is never duplicated by a refresh.

**Read state and filters**

- **FR-017**: Every notification MUST be unread for each person who may see it when it is created.
- **FR-018**: Read state MUST be per person and MUST persist across reloads, sessions, tabs and devices.
- **FR-019**: Opening the notification list MUST NOT mark anything read. Selecting a notification's destination MUST mark that notification read.
- **FR-020**: A person MUST be able to mark an individual notification read or unread, and to mark every unread notification they may see read in one action, including those not loaded on the current page.
- **FR-021**: The list MUST offer All, Unread and Read filters with counts; every notification MUST belong to exactly one of Unread or Read, the default MUST be All, and a chosen filter MUST NOT be reset by an arriving notification or a read-state change.
- **FR-022**: Marking read or unread and loading a new notification MUST update the list, filter counts and badge without a reload.
- **FR-023**: Each filter MUST have a specific empty state; an empty Unread filter MUST be distinguishable from an empty Read filter and from having no notifications at all.
- **FR-024**: Notification times MUST be shown in the person's local time zone in the same relative/absolute style as the rest of the interface, with same-time notifications kept in a stable order.

**Visibility, lifecycle and retention**

- **FR-025**: The list and badge MUST include only notifications for agents the signed-in person may currently see.
- **FR-026**: When a person's access to an agent is revoked, that agent's notifications MUST leave their list and count on the next refresh, and MUST NOT be returned by any route while the person lacks access.
- **FR-027**: Deleting an agent MUST remove its notifications and every person's read state for them.
- **FR-028**: Notifications MUST be retained for 30 days and then no longer listed or counted, so storage and attention stay bounded.
- **FR-029**: The first time the feature is available, historical notable events MUST NOT appear as unread; only events after activation notify.
- **FR-030**: Two people who may see the same agent MUST have independent read states for the same notification.

**Home replacement**

- **FR-031**: Home MUST NOT present the “Since you were away” panel at any viewport, and MUST use the space it occupied rather than leaving an empty column.
- **FR-032**: Every category the removed panel covered MUST be reachable through Notifications; the removal MUST NOT make an event category disappear from the product.
- **FR-033**: The notification list MUST be the single notification inbox for all agents; Home MUST NOT keep a second copy of the same feed.

**Freshness, failure handling and accessibility**

- **FR-034**: While the app is open, a new notification MUST update the badge within 30 seconds without a manual reload, and MUST appear at the top of an open list when the current filter matches.
- **FR-035**: Read-state changes made in one tab or window MUST reach other open tabs and windows within 30 seconds without a reload.
- **FR-036**: If notifications cannot be loaded, the list MUST show an error state with retry, keep the selected filter, and MUST NOT present an empty inbox or a false zero badge when the last successful load was not zero.
- **FR-037**: The entry, badge, filters, rows and read actions MUST be keyboard operable, have meaningful accessible names, keep focus contained while the list is open, close on Escape and restore focus to the entry.
- **FR-038**: Failure and unread state MUST be conveyed by text or icon in addition to colour.
- **FR-039**: The notification list MUST remain usable on a phone viewport, including the same filters, read actions and destinations.

### Key Entities *(include if data involved)*

- **Notification**: a notable event presented to people. It has a stable identity, a category (agent start failure, task run, dream cycle, background job, integration change or channel replies), the agent it concerns, when it happened, a summary derived from recorded facts, a failure tone and a destination.
- **Read state**: one person's read/unread choice for one notification, with when it changed. It is private to that person and never changes another person's view.
- **Notification category**: the kind of notable event, which determines the wording, failure tone and destination. It is not a filter in this release.
- **Agent visibility**: the existing rule that decides which agents a person may see. It governs both the notifications and the unread count, and is evaluated whenever the list is read.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: From any screen, a person can see whether they have unread notifications without navigating, and can reach the notification list in one action.
- **SC-002**: In 100% of checks, the unread badge equals the number of unread notifications in the All list for the same person and visibility, with the capped display still consistent with the true count.
- **SC-003**: Read and unread choices are identical after a reload, in a new tab and on another device in 100% of checks.
- **SC-004**: A newly created notification changes the badge within 30 seconds while the app is open, with no manual reload.
- **SC-005**: Switching between All, Unread and Read shows the correct set in under 1 second for 95% of loads, with no notification appearing in more than one state.
- **SC-006**: “Mark all read” clears every unread notification the person may see and the badge in one action, 100% of the time.
- **SC-007**: Zero notifications from agents a person may not see are listed or counted, and revocation removes them on the next refresh (within 30 seconds while the app is open).
- **SC-008**: After an agent is deleted, zero notifications or read states for that agent remain.
- **SC-009**: Each of the six notable event categories produces exactly one notification at the correct moment with a working destination, 100% of the time.
- **SC-010**: Home presents no “Since you were away” section, and 100% of the event categories it used to show are available through Notifications.
- **SC-011**: Work that is still running produces zero notifications, and a single start-failure episode produces no duplicate notifications.
- **SC-012**: No notification older than the 30-day retention period is listed or counted.
- **SC-013**: One person's read choice never changes another person's state for the same notification, in 100% of checks.
- **SC-014**: Every notification control is reachable and operable by keyboard and has a meaningful assistive-technology name in 100% of accessibility checks.

## Assumptions

- **In-app only**: notifications are shown inside the WebUI. Email, desktop push, sound, Discord/Telegram delivery and other external channels are out of scope.
- **Global, not per-agent**: the inbox spans every agent the person may see. It is not a per-agent page and does not change with the agent selected in the navigation.
- **Coverage means parity**: the six existing notable-event categories are the notification categories for this release. New categories (for example mentions, provider changes or memory changes) are not added.
- **Completed work only**: the old panel could mention a background job “still running”; the inbox waits until the outcome is terminal. A running task or job is not a notification.
- **Explicit read state**: opening the list does not mark anything read; selecting a notification does. Marking unread is supported so the Unread filter is reversible.
- **Mark all is global to the inbox**: it applies to every unread notification the person may see, not only the loaded page or the current filter.
- **Default filter**: All is the default. Unread and Read are one action away and show counts.
- **Retention**: notifications and read states older than 30 days are removed. A person who has been away longer sees an empty or recent inbox rather than an unbounded backlog.
- **No unread backfill**: when the feature first ships, events already in the product's history are not turned into unread notifications. The badge begins at zero and grows from activation.
- **Own actions included**: a notification is shown even when the person caused the change (for example an integration edit), matching the old panel, because it is also a record of what happened.
- **Visibility follows existing access**: a person sees a notification if they can see its agent. The rule is checked on every read, not only when the notification is created. A person who gains access to an agent later sees that agent's retained notifications as unread unless they mark them read.
- **Summary text is factual**: wording is built from stored event facts (task title, state, entry counts, who changed what). Notifications never contain model-written text and never reveal more than the destination page would.
- **Previous-visit window retired for this surface**: the notification list is not bounded by the person's previous visit. Any last-visit tracking that remains is not used to hide notifications.
- **Desktop and phone**: the entry lives in the navigation sidebar and, on a phone, in the same sidebar shown in the navigation drawer. A separate top-bar notification control is not part of this release.
- **No notification preferences**: per-category or per-agent muting, grouping, search and bulk selection are out of scope. Every eligible event notifies every person who may see it.
- **Dependencies**: the existing notable-event sources (agent health, task runs, dream cycles, background jobs, integration changes and channel reply activity), the existing agent-visibility and permission rules, and the app's design system for the navigation entry, badge, list and icons.
