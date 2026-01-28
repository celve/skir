# skir

A terminal UI for managing Claude Code plugins and skills.

## Features

- **Install plugins** from any Git repository (GitHub, GitLab, or any git host)
- **Browse installed plugins** and their skills
- **Link/unlink skills** to Claude Code's skills directory
- **Update plugins** by pulling latest changes
- **Search** through plugins and skills in real-time

## Installation

### From source

Requires Rust toolchain.

```bash
# Clone the repository
git clone https://github.com/anthropics/skir.git
cd skir

# Build and install to ~/.local/bin
make install

# Or install to a custom location
make install PREFIX=/usr/local
```

### Uninstall

```bash
make uninstall
```

## Usage

Run `skir` to launch the interactive TUI.

### Plugin List View

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Ctrl+d` | Scroll down (10 items) |
| `Ctrl+u` | Scroll up (10 items) |
| `Enter` / `l` | View plugin's skills |
| `i` | Install new plugin |
| `d` | Delete plugin |
| `u` | Update plugin |
| `r` | Refresh plugin list |
| `/` | Search |
| `q` | Quit |

### Skill List View

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Ctrl+d` | Scroll down (10 items) |
| `Ctrl+u` | Scroll up (10 items) |
| `l` | Toggle link/unlink skill |
| `h` / `Esc` | Back to plugin list |
| `/` | Search |
| `q` | Quit |

### Install Mode

Press `i` from the plugin list to enter install mode, then paste or type a Git URL.

| Key | Action |
|-----|--------|
| `Enter` | Install plugin |
| `Esc` | Cancel |
| `Backspace` | Delete character (or cancel if empty) |

## Supported URL Formats

skir accepts multiple URL formats for installing plugins:

```bash
# HTTPS
https://github.com/owner/repo
https://gitlab.com/owner/repo

# SSH
git@github.com:owner/repo
git@gitlab.com:owner/repo

# Shorthand (defaults to GitHub)
owner/repo
```

## Directory Structure

skir uses the following directories:

| Directory | Purpose |
|-----------|---------|
| `~/.cache/skir/repos/` | Plugin cache (organized by host/owner/repo) |
| `~/.claude/skills/` | Linked skills (symlinks to skill directories) |

## Skill Discovery

skir automatically scans installed plugins for `SKILL.md` files. Each skill directory containing a `SKILL.md` file can be linked to Claude Code.

Skills use qualified names (`owner:repo:skill-name`) to avoid collisions between plugins.

## Development

```bash
# Debug build
make build

# Release build
make release

# Clean build artifacts
make clean
```

## License

MIT
