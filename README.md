# Connect Four
A Connect Four game with an animated terminal interface, configurable via CLI, supporting multiple simultaneous game instances.

## Features
- **Cross-Platform support:** Runs on Windows, macOS, and Linux via `crossterm`.
- **Fully graphical TUI:** Terminal user interface (TUI) with keyboard-driven controls.
- **Animated disks:** Animated physics-based falling disks (supports resizing).
- **Configurable via CLI:** Gravity, FPS/VFR, player colors, keybindings, and more.

## Game Features
- **Connect Four:** A standard 7x6 game board with two players.
- **Multi-game instancing:** Run multiple independent games side by side in the same terminal.
- **Score counting:** Automatically keeps track of score between the two players.
- **Undo/Redo:** You can undo/redo a move with a single-timeline undo/redo history.
- **Multiple control schemes:** Control using column numbers 1-7 or use WASD/Arrow keys.

## Keybinds

### Default

| Action | Keys |
|--------|------|
| Move left | `←` `A` |
| Move right | `→` `D` |
| Drop disk | `Enter` `Space` |
| Undo | `U` |
| Redo | `Shift+U` |
| Soft reset | `R` |
| Hard reset | `Ctrl+Shift+R` |
| Quit | `Q` |
| Drop in column 1–7 | `1` – `7` |
| Add board | `+` |
| Remove board | `-` |
| Switch to board N | `Ctrl+N` |
| Cycle board left | `Ctrl+A` `Ctrl+←` |
| Cycle board right | `Ctrl+D` `Ctrl+→` |

### Custom Binds

The following actions can be rebound using `--bind <action>=<keys>`:

| Action | Argument |
|--------|----------|
| Move left | `move-left` |
| Move right | `move-right` |
| Drop disk | `drop` |
| Undo | `undo` |
| Redo | `redo` |
| Soft reset | `reset` |
| Quit | `quit` |

Multiple keys can be bound to the same action by separating them with commas.

**Example:**
```
connectfour --bind move-left=h,left --bind move-right=l,right --bind undo=z --bind drop=enter,space
```

**Available key names:** single characters (`a`–`z`, `A`–`Z`, `0`–`9`), or `enter`, `space`, `left`, `right`, `up`, `down`, `esc`, `backspace`, `tab`.

> Note: hard reset, board management, and column number keys cannot be rebound.
