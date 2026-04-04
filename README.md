# tag-tracker

A CLI time tracker with [Waybar](https://github.com/Alexays/Waybar) integration. Track time across multiple activities, visualize the active one in your status bar, and calculate daily earnings in Chilean Pesos (CLP).

Built for managing multiple jobs or projects where you need clear visibility into how many hours you dedicate to each one.

## Features

- **Multiple trackers** with custom name, color, and hourly rate
- **Contract trackers** — define monthly salary and weekly hours, rate is calculated automatically
- **One active at a time** — activating one automatically pauses the previous
- **Waybar module** — shows active tracker with color and elapsed time in a pill badge
- **Tooltip summary** — hover to see today's breakdown per tracker with earnings (pango markup, aligned columns)
- **Keyboard shortcuts** — `SUPER ALT CTRL + 1-9` to activate trackers, `+ 0` to pause
- **Click to pick** — click the waybar module to open a Walker menu, select a tracker to activate or click the active one to pause
- **Date reports** — view status for any specific date with `--date DD/MM/YYYY`
- **Auto-pause on shutdown** — integrates with Hyprland's `exec-shutdown`
- **Stale session recovery** — detects and closes sessions from previous days on startup
- **Daily earnings** — calculates how much you've earned based on hours worked

## Installation

### Build from source

```bash
git clone https://github.com/your-user/tag-tracker.git
cd tag-tracker
cargo build --release
cp target/release/tag-tracker ~/.local/bin/
```

### Requirements

- Rust 1.80+
- Waybar (for status bar integration)
- Hyprland (for keyboard shortcuts and auto-pause on shutdown)

## Usage

### Managing trackers

```bash
# Create freelance trackers (direct hourly rate)
tag-tracker tracker add "Work A" --color "#55a555" --rate 15000
tag-tracker tracker add "Side Project" --color "#a55555"

# Create contract trackers (salary + weekly hours, rate auto-calculated)
tag-tracker tracker add "Esencial" --color "#3388ff" --contract --salary 2500000 --weekly-hours 45
# → Rate: $2.500.000 / (45 × 4.33) = $12.830/hr

# List all trackers (shows type, assigned shortcuts)
tag-tracker tracker list

# Edit a freelance tracker
tag-tracker tracker edit "Work A" --rate 18000
tag-tracker tracker edit "Work A" --new-name "Company A" --color "#66b666"

# Edit a contract tracker (rate recalculates automatically)
tag-tracker tracker edit "Esencial" --salary 3000000
tag-tracker tracker edit "Esencial" --weekly-hours 40

# Change a tracker's keyboard shortcut (1-9)
tag-tracker tracker edit "Work A" --shortcut 5

# Delete a tracker
tag-tracker tracker delete "Side Project"
```

### Keyboard shortcuts

Each tracker is automatically assigned a keyboard shortcut (`1-9`) when created. Shortcuts integrate with Hyprland:

| Shortcut | Action |
|----------|--------|
| `SUPER ALT CTRL + 1-9` | Activate the tracker assigned to that number |
| `SUPER ALT CTRL + 0` | Pause the active tracker |

Switching trackers is instant — pressing a different shortcut pauses the current tracker and activates the new one.

```bash
# Reassign a shortcut
tag-tracker tracker edit "Work A" --shortcut 5

# Manually sync shortcuts with Hyprland (usually automatic)
tag-tracker sync-keybindings
```

Shortcuts are synced to `~/.config/hypr/tag-tracker-bindings.conf` automatically on every `tracker add`, `edit`, or `delete`.

### Time tracking

```bash
# Start tracking — pauses any currently active tracker
tag-tracker activate "Work A"

# Switch to another — Work A gets paused, Work B starts
tag-tracker activate "Work B"

# Pause the active tracker
tag-tracker pause
```

### Viewing status

```bash
# Today's status (all trackers)
$ tag-tracker status
● Active: CobraYa
  Contract:   $1.000.000/mo · 15h/wk
  Time today: 1m
  Earned:     $334

◉ Paused: Empresa B
  Time today: 3h 52m
  Earned:     $77.589

◉ Paused: Esencial
  Contract:   $2.500.000/mo · 45h/wk
  Not started today

───────────────────────────────────
Total today: 3h 53m | $77.923

# Report for a specific date (DD/MM/YYYY)
$ tag-tracker status --date 01/04/2026
Report for 01/04/2026

◉ Paused: Work A
  Time: 3h 00m
  Earned:     $45.000

───────────────────────────────────
Total: 3h 00m | $45.000

# Filter by tracker name (works with or without --date)
$ tag-tracker status "Work A"
$ tag-tracker status --date 01/04/2026 "Work A"
```

## Waybar Integration

### Module config

Add to `~/.config/waybar/config.jsonc`:

```jsonc
// Add "custom/tag-tracker" to your modules-left (or wherever you prefer)
"modules-left": ["custom/omarchy", "hyprland/workspaces", "custom/tag-tracker"],

// Module definition
"custom/tag-tracker": {
    "exec": "tag-tracker waybar",
    "return-type": "json",
    "format": "{}",
    "interval": 5,
    "signal": 11,
    "tooltip": true,
    "on-click": "tag-tracker menu"
}
```

> **Note:** Choose a signal number that doesn't conflict with your other custom modules.

### Styles

Add to `~/.config/waybar/style.css`:

```css
@keyframes border-pulse {
    0%   { border-color: alpha(@foreground, 0.0); }
    50%  { border-color: alpha(@foreground, 0.25); }
    100% { border-color: alpha(@foreground, 0.0); }
}

#custom-tag-tracker {
    margin: 0 6px;
    padding: 0 10px;
    border-radius: 10px;
    border: 2px solid transparent;
    transition: all 0.3s ease;
}

#custom-tag-tracker.active {
    background-color: rgba(255, 255, 255, 0.07);
    animation: border-pulse 5s ease-in-out infinite;
}

#custom-tag-tracker.active:hover {
    background-color: rgba(255, 255, 255, 0.12);
}

#custom-tag-tracker.idle {
    opacity: 0.35;
    font-size: 10px;
}
```

The active tracker's color is applied to the text via pango markup, with a subtle semi-transparent pill background from CSS.

### Auto-pause on shutdown

Add to `~/.config/hypr/hyprland.conf`:

```
exec-shutdown = tag-tracker pause
```

This ensures the active session is properly closed when you log out or shut down.

## How it works

```
SUPER ALT CTRL + 1  (or: tag-tracker activate "Work A")
  │
  ├─ Pauses current active tracker (if any)
  ├─ Creates a new session (started_at = now)
  ├─ Sets tracker state to "active"
  └─ Signals waybar for instant refresh (pkill -RTMIN+N waybar)

Waybar polls "tag-tracker waybar" every 5 seconds
  │
  └─ Returns JSON: { text, tooltip, class }
       ├─ text: pango markup with tracker color, name, elapsed time
       ├─ tooltip: daily summary of all trackers with earnings
       └─ class: "active" or "idle" (idle hides the module)

Keyboard shortcuts are stored in ~/.config/hypr/tag-tracker-bindings.conf
  │
  └─ Auto-synced on tracker add/edit/delete
       ├─ Writes bindd entries for each tracker with a shortcut
       ├─ Sources itself into hyprland.conf (one-time, idempotent)
       └─ Reloads Hyprland config via hyprctl reload
```

## Data storage

- **Database:** `~/.local/share/tag-tracker/tag-tracker.db` (SQLite)
- WAL mode enabled for performance
- Foreign keys enforced
- Schema migrations via `PRAGMA user_version`

## Tech stack

- **Rust** (edition 2024)
- **clap** — CLI argument parsing with derive macros
- **rusqlite** — SQLite with bundled feature
- **chrono** — Time handling
- **serde / serde_json** — JSON serialization for waybar output
- **colored** — Terminal color output
- **directories** — XDG-compliant data paths
- **thiserror** — Error type definitions

## License

MIT
