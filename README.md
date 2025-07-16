# hrdza_log

A modular, terminal-based log analyzer written in Rust for linux(debian).

## Prerequisites

- **Operating System**: Linux (tested on Kubuntu)
- **Rust Toolchain**: Rust 1.70+ and Cargo
- **System Services**:
  - **systemd** (for journal logs)
  - **auditd** (for audit events)
  - **udev** (for device events)
  - **X11** (for active window tracking)
  - **PulseAudio** (for audio stream monitoring)
  - **CUPS** (for printer status)
- **External Tools**:
  - `lpstat` (CUPS queries)
  - `pactl` (PulseAudio queries)

## Installation of prerequisites

```bash 

sudo apt update
# Install build tools and Rust (if not already installed)
sudo apt install -y build-essential curl

# Fonts for the pdf file
sudo apt install fonts-dejavu-core


# (Optional) If you don't have Rust & Cargo yet, install via rustup:
# curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# source $HOME/.cargo/env

# Install system libraries for each watcher:
sudo apt install -y \
    libsystemd-dev \      # for systemd journal access
    libudev-dev \         # for udev device monitoring
    libx11-dev \          # for X11 active window tracking
    libpulse-dev \        # for PulseAudio stream queries
    libcups2-dev \        # for CUPS printer queries
    auditd \              # for Linux audit events
    pkg-config \          # for building native crates

```

## Installation of hrdza

```bash
git clone repo.url
cd hrdza_log
cargo build 
```

## Usage

Run the application:

```bash
cargo run
```

## Features & Controls

Within the TUI, you can:

- **Filter Logs**  
  - `f`: Edit **keyword** filter  
  - `d`: Edit **date** filter (YYYY-MM-DD)  
  - `u`: Edit **user** filter  
  - `p`: Edit **process** filter  
  - `l`: Edit **level** filter (INFO/WARN/ERR)  
  - `i`: Edit **source IP** filter  
  - `c`: **Clear** all filters  

- **Visualizations**  
  - `h` or `g`: Toggle **heatmap** (7×24 activity matrix)  
  - **Barplot** by log level (in “Exploratory” mode)  
  - **Color-coded** log lines: ERROR (red), WARN (yellow), INFO (default)  

- **Application Usage**  
  - `a`: Show **active window usage** summary (hours per app)  

- **Search**  
  - `x`: **Fuzzy search** through messages via `skim`  

- **Export**  
  - `e`: **Export** current view to PDF (`hrdza_log_report.pdf`)  

- **Exit**  
  - `q` or **Ctrl-C**: Quit the application  

## File Overview

- **Cargo.toml**: project metadata & dependencies  
- **src/main.rs**: application entry point, spawns watchers and TUI  
- **src/db.rs**: SQLite schema, temporal index, FTS5, log insertion  
- **src/sources.rs**: watches files -> "/var/log/auth.log", "/var/log/syslog", "/var/log/kern.log", "/var/log/daemon.log", "/var/log/dpkg.log", "/var/log/messages"
- **src/audit.rs**: streams auditd events via Netlink  
- **src/journal.rs**: streams systemd journal entries  
- **src/filters.rs**: multi-criteria log filters  
- **src/graphs.rs**: heatmap calculation  
- **src/devices.rs**: monitors udev, CUPS, PulseAudio  
- **src/apps.rs**: tracks active window usage via X11  
- **src/export.rs**: PDF export using `genpdf`  
- **src/ui.rs**: terminal UI with multiple modes and charts  


### Exemple of usage: 

In the HRDZA_log TUI all filters stack with a logical AND, and you set them one-by-one:

    Press `d` to edit the date filter.

    Type the date in YYYY-MM-DD format (e.g. XXXX-XX-XX) and hit Enter.

    Press `u` to edit the user filter.

    Type username and hit Enter.

### Delete a single filter in edit mode

Simply reopen this filter and confirm without typing anything.






## License

Creative Commons Zero v1.0 Universal
