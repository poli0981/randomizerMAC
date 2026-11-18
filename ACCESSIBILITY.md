# Accessibility

**Last reviewed**: 2026-11-18 · **Applies to**: RandomMac 2.1.x (Windows and Linux apps, CLI)

RandomMac should work for everyone who needs to change or restore a MAC address or DNS settings,
whatever input device, screen reader, display setting or language they use. This statement covers
our goals, what works today, how it was tested, what is known not to work yet, and how to report a
barrier.

RandomMac is a one-person project (see [DISCLAIMER.md](DISCLAIMER.md)). This page describes honest
best effort, not a certified result.

## Goals

- **Target: [WCAG 2.2](https://www.w3.org/TR/WCAG22/) level AA.** The Windows and Linux apps share
  one interface, so the target covers both. It is a goal, not a claim of conformance: there has been
  no formal audit.
- **Priorities, in order:**
  1. Every task works with the keyboard alone: changing and restoring a MAC address or DNS
     settings, the history, the settings and the blacklist.
  2. Every control has a name, role and state that screen readers announce, in the interface
     language.
  3. Content stays readable with zoom, light and dark themes, reduced motion and Windows contrast
     themes.
- **Accessibility regressions are bugs.** They are triaged and fixed like any other bug (see
  [Reporting accessibility issues](#reporting-accessibility-issues)).

## What works today

### Keyboard

- Buttons, links and form controls are native elements. No element has a positive `tabindex`.
- **The adapter list** on the **Dashboard** is a list box: ↑ ↓ Home End move between adapters, and
  Enter or Space selects one.
- **Skip to content** is the first Tab stop. After you move to another page, focus moves to that
  page's heading.
- Every focused control shows a visible focus indicator.
- **Confirmation dialogs** are native dialogs:
  - focus starts on **Cancel**, the safe choice;
  - focus stays inside them while they are open;
  - Escape closes them;
  - focus then returns to the button that opened them.
- **When the focused control goes away**, focus moves to a sensible place nearby. This covers
  closing a banner or a notification, removing a blacklist rule and clearing the history.
- **While a change runs**, **Randomize**, **Apply** and **Restore original** stay focusable. They
  are marked unavailable (`aria-disabled`) instead of being disabled, so focus is not lost.
- **Scroll areas** (the history table, the log, the latency table, the update notes) take focus
  and scroll with the arrow keys. They are labelled regions.
- Shortcuts are exposed with `aria-keyshortcuts`. The shortcut hints on buttons are not read as part
  of their names.

### Screen readers

- **Page structure:**
  - landmarks: navigation, main, and a labelled region for notifications;
  - one `h1` per page.
- `<html lang>` follows the interface language from the moment the window opens. Each language name
  in the language picker has its own `lang`.
- **Changes:**
  - The progress of a MAC or DNS change is announced step by step in one live region, for example
    "Step 2 of 4: …".
  - The current step is marked with `aria-current="step"`.
  - The result of a change is announced once.
- **Notifications in the window:**
  - there are no nested live regions;
  - error notifications stay until you close them;
  - the others close on their own, and pause while you hover over or focus them;
  - each close button is named after its notification.
- **Updates:** the update banner is a status message, the update dot in the sidebar has text for
  screen readers, and the progress bar on the **Update** page is labelled.
- In the adapter list, a busy adapter that isn't the selected one says so in text.
- Switches use `role="switch"`. Their description and hint are linked with `aria-describedby`.
- The **History** table has a caption. Error details are visible text, not only a tooltip.
- **The DNS page:**
  - Unavailable encryption options are disabled, and their reason is the option's description.
  - Field errors are linked to their fields (`aria-invalid`, `aria-describedby`).
  - The latency test announces its progress and its result in a live region.
  - Each copy button names the endpoint it copies.
- Decorative icons are hidden from assistive technology, and icon-only buttons have names.
- System notifications, for example after an action from the tray, are translated and include the
  error code when the action fails.

### Visual

- **Themes:** light, dark and system.
  - Text colours, including subtle text, accent text and placeholders, were checked at 4.5:1 or
    more in both themes.
  - The borders of inputs and selects, and the switch track, were checked at 3:1 or more.
- **Windows contrast themes** (`forced-colors`): switches, the selected adapter, the current page in
  the sidebar, selected chips, the update dot, progress bars and primary buttons use system colours.
- **Reduced motion** (`prefers-reduced-motion`): animations stop. Spinners and indeterminate
  progress bars stand still.
- **No meaning from colour alone:** the adapter's connection dot comes with a state text, and
  results are given in text.

### Zoom

- Ctrl+= zooms the window in, Ctrl+− zooms it out and Ctrl+0 resets the zoom.
- The sidebar scrolls when the window is zoomed in.
- The layout was checked at 200 %.
- The smallest window is 880 × 600.

### Language

The interface is available in English and Tiếng Việt, including the screen-reader texts, the tray
menu and notifications. The Vietnamese texts are AI-generated translations. Fixes are welcome through
the [Language / Translation Fix](https://github.com/poli0981/randomizerMAC/issues/new?template=fix_lang.yml)
issue template.

## Keyboard reference

| Keys | Where | Does |
| --- | --- | --- |
| Tab / Shift+Tab | Everywhere | Next / previous control |
| Enter on **Skip to content** | First Tab stop | Jump to the page content |
| Ctrl+1 … Ctrl+8 | Everywhere | Open the pages in sidebar order: Dashboard, DNS, History, Settings, Log, Update, About, Social |
| F5 | Everywhere | Refresh the adapter list |
| Ctrl+R | Dashboard | **Randomize**: preview a new address |
| Ctrl+Enter | Dashboard | **Apply** the new address |
| ↑ ↓ Home End | Adapter list | Move between adapters |
| Enter / Space | Adapter list | Select the adapter |
| Escape | Confirmation dialog | Close it; focus returns to the button that opened it |
| Ctrl+= / Ctrl+− / Ctrl+0 | Everywhere | Zoom in / zoom out / reset the zoom |

## How we test

### Tested with

| Date | Version | Platform | Method | Result |
| --- | --- | --- | --- | --- |
| 2026-11 | 2.0.2 | Windows 11, Windows app (WebView2) | Narrator; keyboard only, with the mouse turned off (maintainer) | No blocking issues |
| Every pull request | — | jsdom in CI | [axe-core](https://github.com/dequelabs/axe-core) on every page (DNS included) and on an open confirmation dialog; role and name checks with Testing Library (`src/lib/a11y.test.ts`) | Must pass |

The automated scan runs in jsdom, which has no layout engine, so it can't measure colour contrast.
Contrast is checked by hand.

**Not tested yet** (reports are very welcome):

- **Screen readers:** NVDA and JAWS (Windows); Orca with the Linux app (WebKitGTK).
- **Voice control:** Voice Access, Dragon.
- **Display:** Windows contrast themes on real hardware; high-DPI scaling above 200 %.

### Manual check before a release

1. **Keyboard only.** Work through every main flow:
   - Dashboard: pick an adapter, randomize, apply, restore;
   - DNS: choose a provider, apply, restore, run the latency test;
   - History: filter and export;
   - Settings, including the blacklist.

   Check that focus is always visible and never stuck.
2. **Screen reader.** Use Narrator, NVDA or Orca for the same flows. Listen for names, states,
   errors and the progress of a change.
3. **Zoom.** Check at 200 %.
4. **Display settings.** Check with a Windows contrast theme, with reduced motion, and in light and
   dark themes.
5. **Automated scan.** Run `npm test`, which includes the axe-core scan.

## Supported environments

| Surface | Engine | Assistive technology checked |
| --- | --- | --- |
| Windows app (Windows 10/11) | Microsoft Edge WebView2 | Narrator |
| Linux app (Ubuntu 24.04+, Debian 13+, Fedora 41+) | WebKitGTK | Not checked yet |
| Command-line tool (`randommac-cli`) | Any terminal | Output is plain text; not specifically tested |

## Known limitations

These are the barriers we know about, with workarounds where one exists.

1. **The tray menu is a native menu of the operating system.**
   - Windows: press Win+B to reach the notification area.
   - Linux: the tray icon needs StatusNotifier support from the desktop. GNOME needs the
     AppIndicator extension.

   *Workaround:* the main window does what the tray menu does, with two exceptions:
   - **Restore all adapters**: restore the adapters one by one on the **Dashboard**.
   - **Quit**: turn off **Keep running in the tray when the window is closed** in **Settings**;
     closing the window then quits RandomMac.
2. **Parts that belong to the operating system.** The UAC prompt when RandomMac restarts as
   administrator, the file dialogs for export and import, and the system notifications are drawn by
   the operating system, so their accessibility is the operating system's.
3. **Wide content in narrow windows.** The latency table and long DNS server lists scroll
   horizontally inside their card. *Workaround:* make the window wider, or collapse the sidebar.
4. **Zoom above 200 % on small screens.** The layout is not designed for it. *Workaround:* collapse
   the sidebar.
5. **Orca.** The Linux app hasn't been tested with Orca yet.
6. **Translations.** The Vietnamese texts, including the screen-reader texts, are AI-generated
   translations; native speakers' fixes are welcome.

## Contributor expectations

Contributors check these for every change to the interface (see also
[CONTRIBUTING.md](CONTRIBUTING.md)):

- **Elements.** Use native buttons, links and form controls. A custom widget, like the adapter list,
  needs the matching ARIA role and full keyboard support. Don't use a positive `tabindex`.
- **Names.** Every control has an accessible name from `src/lib/i18n/en.ts` and `vi.ts`, in both
  languages. Icon-only buttons get `aria-label`, and decorative icons get `aria-hidden`.
- **Focus.** A scroll area that takes focus gets `role="region"` and a label.
- **Motion and colour.** Animations run only under `prefers-reduced-motion: no-preference`, or are
  switched off under `reduce`. Don't show meaning with colour alone.
- **Announcements.** Use the existing live regions, and keep announcements short.
- **Testing.**
  - `npm test` must pass, including the axe-core scan in `src/lib/a11y.test.ts`. Extend the scan
    for new pages and dialogs.
  - Do a keyboard-only pass of what you changed and say so in the pull request.
- **This file.** Update it when behaviour or a known limitation changes.

## Reporting accessibility issues

Open an issue with the
**[Accessibility issue](https://github.com/poli0981/randomizerMAC/issues/new?template=accessibility.yml)**
form. Include:

- what you were trying to do and what happened instead;
- your assistive technology and its version;
- the RandomMac version (on the **About** page, or `randommac-cli --version`) and your operating
  system.

You don't need to know the cause.

If you can't or would rather not use GitHub, email **accessibility@poli0981.dev** with
"Accessibility" in the subject. Security problems go through [SECURITY.md](SECURITY.md) instead.

### Severity

| Severity | Meaning | Example |
| --- | --- | --- |
| Critical | Blocks a core task, with no workaround | You can't change or restore a MAC address or DNS settings with your keyboard or screen reader |
| High | Major difficulty, but a workaround exists | An error is not announced, but it is shown on screen |
| Medium | Confusing, but the task is possible | A wrong or missing label; focus lands in an odd place |
| Low | Minor annoyance | A verbose announcement; an untranslated hint |

### How we respond

- We thank you and ask questions if something is unclear. We never ask you to justify why you need
  an accessibility feature.
- We add the `accessibility` label, set a severity, and triage blockers before cosmetic issues.
- We share a workaround as soon as we know one.
- When a fix is ready, we ask you to confirm it, if you are willing. Fixes are listed in
  [CHANGELOG.md](CHANGELOG.md).

### Response times

This is best effort from a one-person project:

| Severity | First response | Fix released |
| --- | --- | --- |
| Critical | within 7 days | next patch release |
| High | within 14 days | next release |
| Medium / Low | within 30 days | when practical, usually the next minor release |

## Ownership and maintenance

The sole maintainer, [@poli0981](https://github.com/poli0981), owns accessibility. That covers:

- triaging reports;
- tracking them with the `accessibility` label;
- running the [manual check](#manual-check-before-a-release) before releases;
- keeping this statement current, including its review date.

## Feedback

Suggestions for this statement are welcome as an issue or a pull request.

A [Vietnamese translation](docs/i18n/vi/ACCESSIBILITY.md) is available. If the two differ, this
English version prevails.

This statement follows the structure recommended by the Open Source Guides
([Accessibility best practices for your project](https://opensource.guide/accessibility-best-practices-for-your-project/#start-with-an-accessibility-statement))
and the
[Open Source Accessibility toolkit](https://github.com/open-source-accessibility/accessibility-toolkit/blob/main/ACCESSIBILITY.md).
