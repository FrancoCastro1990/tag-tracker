# tag-tracker

A CLI time tracker with [Waybar](https://github.com/Alexays/Waybar) integration. Track time across multiple activities, visualize the active one in your status bar, and calculate daily earnings in Chilean Pesos (CLP).

Built for managing multiple jobs or projects where you need clear visibility into how many hours you dedicate to each one.

## Features

- **Multiple trackers** with custom name, color, and hourly rate
- **One active at a time** — activating one automatically pauses the previous
- **Waybar module** — shows active tracker with background color and elapsed time
- **Tooltip summary** — hover to see today's breakdown per tracker with earnings
- **Keyboard shortcuts** — `SUPER ALT CTRL + 1-9` to activate trackers, `+ 0` to pause
- **Click to pause** — single click on the waybar module pauses tracking
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
# Create trackers
tag-tracker tracker add "Work A" --color "#55a555" --rate 15000
tag-tracker tracker add "Work B" --color "#5555a5" --rate 20000
tag-tracker tracker add "Side Project" --color "#a55555"

# List all trackers (shows assigned shortcuts)
tag-tracker tracker list

# Edit a tracker
tag-tracker tracker edit "Work A" --rate 18000
tag-tracker tracker edit "Work A" --new-name "Company A" --color "#66b666"

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
$ tag-tracker status
● Active: Work B
  Time today: 2h 34m
  Earned:     $51.333

◉ Paused: Work A
  Time today: 1h 15m
  Earned:     $22.500

───────────────────────────────────
Total today: 3h 49m | $73.833
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
    "on-click": "tag-tracker pause"
}
```

> **Note:** Choose a signal number that doesn't conflict with your other custom modules.

### Styles

Add to `~/.config/waybar/style.css`:

```css
#custom-tag-tracker {
    margin: 0 4px;
    padding: 0 8px;
    border-radius: 4px;
    transition: all 0.3s ease;
}

#custom-tag-tracker.active {
    font-weight: bold;
}
```

The tracker's background color is applied dynamically via pango markup with automatic contrast detection for the foreground text.

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
