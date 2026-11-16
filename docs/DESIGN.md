# Design

The desk is a working mail client. The lagoon navigation rail is the color commitment. The work surface is a cool mist. Brass appears on the mark, the active folder icon, and the compose button when it is hovered.

Tokens live in `frontend/src/styles/tokens.css`. Public Sans is self-hosted through `@fontsource/public-sans`.

## Light

| Token | Value | Use |
| --- | --- | --- |
| Lagoon | `#0e3b3a` | Sidebar |
| Lagoon ink | `#e7f4f2` | Text on the sidebar |
| Lagoon muted | `#a9cfc9` | Secondary sidebar text |
| Brass | `#f2c14e` | Mark, active icon, compose hover |
| Brass ink | `#1a1408` | Text on brass |
| Canvas | `#e6eeef` | Desk background |
| Surface | `#f4f8f8` | List, top bar, menus |
| Ink | `#10211f` | Primary text |
| Muted | `#3e5552` | Secondary text |
| Line | `#d3e0e2` | Borders |

## Dark

The rail deepens to `#082221`. The canvas is `#0c1413` and the surface is `#141d1c`. Ink is `#e7f3f1`. Muted text is `#9bb5b1`. Brass stays `#f2c14e`.

## Shape

Controls use a 10px radius. Menus use 14px. Compose is a pill. Shadows are a 1px offset plus a soft blur, stored as `--km-shadow`. Focus rings are brass. The rail width eases between 248px and 72px. Below 960px the rail becomes a drawer and the reading pane waits until a message exists.
