# claude-chill

A PTY proxy that reduces terminal flicker by truncating synchronized output blocks.

## The Problem

Claude Code (and similar tools) use synchronized output (`\x1b[?2026h` / `\x1b[?2026l`) to update the terminal atomically. When these blocks contain thousands of lines, some terminals struggle to render them smoothly, causing visible flicker.

## The Solution

claude-chill sits between your terminal and the child process, intercepting synchronized output blocks and truncating them to a configurable number of lines. This keeps atomic updates small enough for smooth rendering while preserving full history for lookback.

## Installation

```bash
cargo install --path crates/claude-chill
```

## Usage

```bash
claude-chill <command> [args...]

# Example: wrap Claude Code
claude-chill claude

# Example: wrap any command
claude-chill bash
```

### Environment Variables

- `CHILL_MAX_LINES` - Max lines per sync block (default: 100)
- `CHILL_HISTORY` - Max history lines for lookback (default: 100000)

### Lookback Mode

Press `Ctrl+Shift+PgUp` to dump the full history buffer to the terminal.

## How It Works

1. Creates a PTY pair and spawns the child process
2. Intercepts all output from the child
3. Detects synchronized output blocks (between `?2026h` and `?2026l`)
4. For "full redraw" blocks (containing clear screen + cursor home):
   - Stores full content in history buffer
   - Outputs only the last N lines
5. Passes all other content through unchanged

## License

MIT
