# tag-tracker Design Spec

## Context

Managing multiple jobs without clear visibility into hours dedicated to each one. Need a CLI tool that tracks time per activity ("tracker"), shows the active tracker in waybar, and calculates earnings based on hourly rates in Chilean pesos (CLP). Born from the need to track two jobs but extensible to any activity.

## Overview

A Rust CLI tool called `tag-tracker` that:
- Manages named trackers with color, optional icon, and hourly rate
- Tracks time via activate/pause with only one tracker active at a time
- Displays the active tracker in waybar with real-time elapsed time
- Shows daily summary with hours and earnings in the waybar tooltip
- Stores all data in SQLite for reliability and queryability

## Architecture

**Single binary + waybar polling.** One Rust binary handles all CLI commands AND waybar output. No daemon, no IPC. Waybar calls `tag-tracker waybar` every 5 seconds. After any state change, the CLI signals waybar for instant refresh via `pkill -RTMIN+10 waybar`.

This is consistent with the user's existing waybar custom module pattern (vpn-status, screen recording).

## Data Model

### trackers table

| Column      | Type    | Description                          |
|-------------|---------|--------------------------------------|
| id          | INTEGER | Primary key, autoincrement           |
| name        | TEXT    | Unique tracker name                  |
| color       | TEXT    | Hex color (e.g. "#55a555")           |
| icon_path   | TEXT    | Optional path to icon file           |
| hourly_rate | INTEGER | CLP per hour (e.g. 15000)            |
| state       | TEXT    | "created" / "active" / "paused"      |
| created_at  | TEXT    | ISO 8601 timestamp                   |

### sessions table

| Column     | Type    | Description                              |
|------------|---------|------------------------------------------|
| id         | INTEGER | Primary key, autoincrement               |
| tracker_id | INTEGER | FK to trackers(id)                       |
| started_at | TEXT    | ISO 8601, when activated                 |
| ended_at   | TEXT    | ISO 8601, when paused/switched (NULL=active) |

**Session lifecycle:** Each activate→pause cycle creates one row. `started_at` is set on activation, `ended_at` on pause/switch. To calculate today's time: sum all sessions where date matches today.

## CLI Commands

```
tag-tracker <COMMAND>

COMMANDS:
  tracker add <name> --color <hex> --rate <clp> [--icon <path>]
  tracker list
  tracker edit <name> [--name <new>] [--color <hex>] [--rate <clp>] [--icon <path>]
  tracker delete <name>
  activate <name>       # activates tracker, pauses any active one
  pause                 # pauses the currently active tracker
  status                # daily summary: time per tracker, earnings, totals
  waybar                # JSON output for waybar module
  help                  # show help
```

### Command behavior

**`activate <name>`:**
1. If another tracker is active → pause it (close its session)
2. Set target tracker state = "active"
3. Create new session row (started_at=now, ended_at=NULL)
4. Signal waybar: `pkill -RTMIN+10 waybar`

**`pause`:**
1. Find active tracker
2. Set state = "paused", close current session (ended_at=now)
3. Signal waybar: `pkill -RTMIN+10 waybar`

**`status`:**
- Show each tracker with today's accumulated time and earnings
- Active tracker: `●`, Paused: `◉`, Created: `○`
- Show total at the bottom

**`waybar`:**
- Active tracker: `{"text": " Name  Xh Ym", "tooltip": "daily summary", "class": "active"}`
- No active tracker: `{"text": "", "tooltip": "", "class": "idle"}`
- The tooltip shows all trackers' daily time and earnings

## Waybar Integration

### Module config (config.jsonc)

```jsonc
"custom/tag-tracker": {
    "exec": "tag-tracker waybar",
    "return-type": "json",
    "interval": 5,
    "signal": 10,
    "tooltip": true,
    "on-click": "tag-tracker pause"
}
```

Add `"custom/tag-tracker"` to `modules-left` (after workspaces).

Signal 10 is free (8=screen recording, 9=VPN).

### Styles (style.css)

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

The tracker's color is applied via pango markup in the JSON text output, not CSS.

### Tooltip format

```
● Empresa A: 2h 34m ($38,500)
◉ Empresa B: 1h 15m ($25,000)
──────────
Total: 3h 49m | $63,500
```

## Auto-Pause on Shutdown

Two-layer approach:

1. **Clean shutdown:** Add `exec-shutdown = tag-tracker pause` to `~/.config/hypr/hyprland.conf`
2. **Crash fallback:** On any command execution, detect sessions with `ended_at = NULL` where `started_at` is from a previous calendar day. Close them by setting `ended_at = started_at + accumulated time that day` (or just `ended_at = end of that day 23:59:59` as a simple heuristic).

## Project Structure

```
tag-tracker/
├── Cargo.toml
├── src/
│   ├── main.rs              # entry point, clap CLI setup
│   ├── cli/
│   │   ├── mod.rs           # CLI command definitions (clap derive)
│   │   └── commands.rs      # command handler functions
│   ├── domain/
│   │   ├── mod.rs
│   │   ├── tracker.rs       # Tracker struct, TrackerState enum
│   │   └── session.rs       # Session struct
│   ├── db/
│   │   ├── mod.rs
│   │   ├── connection.rs    # SQLite connection, migrations, PRAGMA setup
│   │   ├── tracker_repo.rs  # CRUD for trackers
│   │   └── session_repo.rs  # CRUD for sessions
│   ├── waybar/
│   │   ├── mod.rs
│   │   └── output.rs        # JSON output generation
│   └── error.rs             # custom error type (thiserror)
```

## Dependencies

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
rusqlite = { version = "0.31", features = ["bundled"] }
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
directories = "5"
colored = "2"
```

## Storage

Database at `~/.local/share/tag-tracker/tag-tracker.db` (XDG standard via `directories` crate). Auto-created on first run.

SQLite settings: WAL mode, foreign keys enabled, `PRAGMA user_version` for schema migrations.

## Verification

1. `cargo build` compiles without errors
2. `cargo clippy` passes with zero warnings
3. `cargo test` — unit tests for:
   - Tracker CRUD operations
   - Session start/stop logic
   - Time calculation (today's total)
   - Earnings calculation
   - Waybar JSON output format
   - Stale session detection
4. Manual test: `tag-tracker tracker add "Test" --color "#55a555" --rate 15000`
5. Manual test: `tag-tracker activate "Test"` → `tag-tracker status` → `tag-tracker pause`
6. Manual test: `tag-tracker waybar` produces valid JSON
7. Waybar integration: add module to config, verify it appears and updates
