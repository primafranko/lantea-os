# Experience — proposals for Phase 7

Status: **proposals, not decisions.** Turn each into a decision record when Phase 7 starts. The mockups made so far are mood references, not the spec.

## Desktop structure — the "Harbour" layout

- **Top bar (thin):** the star (opens the launcher) on the left; date and time in the centre; tray and **Condition** on the right. Condition shows as emblem + one word ("Steady"). Clicking it opens a popover with the Store, the Watch, the Record and Offerings, and "Open Lantea…".
- **Left rail:** icons-only task manager (pinned and running apps), moving out of the way of windows.
- **Desktop:** no icons, no text in the wallpaper. Quotes belong on the lock screen.
- **Meta key:** Plasma Overview.
- **One "Lantea" app** (Kirigami) with pages Condition, Store & Held, Watch, Offerings, Record; the same pages as System Settings modules. Launcher entries for Condition, Tend and Record open those pages. It reads the same `condition.json` as the CLI.
- **Notifications** only when the condition state changes, with the single low bell.
- Alternative closer to stock KDE: one floating bottom panel with Condition in the tray.

## Naming rule in the interface

Every Lantea term appears next to its plain name somewhere visible: in `.desktop` files the Lantea term is `Name` and the plain term `GenericName` (Kickoff shows both); in-app navigation pairs them ("The Watch · Services").

## How to build the theme

- A standard **Plasma Global Theme** (look-and-feel package with `layout.js`) installed system-wide and set as default — replaceable by design. plasma-manager is optional for users who want their desktop declared in Nix.
- Build: colour scheme, Plasma Style (panel/popup SVGs), recoloured Papirus icons, recoloured Breeze window decorations, Konsole profile, login screen theme, lock screen, splash, Plymouth theme, sound theme, fonts.
- **Do not build a custom application style** (QStyle/Kvantum). It is the largest maintenance cost in KDE theming and breaks with Plasma releases. Breeze in Lantea colours gives most of the look.
- **Login screen:** Plasma 6.6's new Plasma Login Manager supports only the Breeze theme. A custom login screen means staying on SDDM. Needs a decision.
- **Day and night:** since Plasma 6.5, Global Themes can switch on the Night Light schedule. One theme, two wallpaper variants: dusk by day, lit lanterns after dark.

## Fonts

Cormorant Garamond only for display moments (login, splash, lock, About, the Condition state word). Inter for all interface text, window titles included. IBM Plex Mono in the terminal.

## Colour roles (contrast measured against ground `#0A0A0C`)

| Role | Colour | Contrast |
|---|---|---|
| Text | `#F2EDE4` | 17.0 : 1 |
| Muted text | `#8A857C` | 5.4 : 1 |
| Focus, selection, brand | aged gold `#C9A227` | 8.2 : 1 (dark text on gold also 8.2 : 1) |
| Links | light Adriatic `#7FA6CF` (new) | 7.8 : 1 |
| Selected-row surface | Adriatic blue `#1B3A5C` | text on it 10.0 : 1 — never use it *for* text (1.7 : 1) |

## Condition colours

The state is always shown as a word; colour only reinforces it (colour-blind safe).

| State | Colour |
|---|---|
| Steady | limestone `#D9CDB8` — neutral, nothing to see |
| Watchful | aged gold `#C9A227` |
| Attention | lamp amber `#E8A33D` |
| Distressed | ember `#D2694F` (new; 5.5 : 1) |

## Screens the mockups are missing or get wrong

- The **disk-unlock PIN** screen in Plymouth (first thing seen each boot).
- Login screen: **Keeper never appears** — it is not a desktop account.
- Updates screen: one Offering per generation, no per-package checkboxes; button "Accept"; "takes effect at next restart"; "you can return to the current state at any time".
- No percentages anywhere (gauges, "(37%)").
- "Recovery Options" on the login screen = reboot into the boot menu of previous generations (`systemctl reboot --boot-loader-menu`).
- Discover offers only Flatpak on NixOS; drop it or limit to declared Flatpaks.
- File manager: Held replaces Trash; "Record" is not a place.
- Wallpapers: 4K masters in 16:9, 16:10 and 21:9 with a quiet area for windows; check licence terms of any generated images before redistributing.
- Name check before anything is published: "Lantea" is also the planet in *Stargate Atlantis*.
