# accentd

Press-and-hold accent character popup for Linux, like macOS.

Hold a letter key (e.g. `e`) for 300ms and a popup appears with numbered accented variants:

<img width="302" height="59" alt="Screenshot From 2026-02-24 15-17-36" src="https://github.com/user-attachments/assets/5ae79ed9-f0bb-4706-be8a-62f45080d57e" />

Press a number to replace the letter. Press ESC to cancel.

Works on any desktop environment and display server: GNOME, KDE, Sway, Hyprland, X11, Wayland. Operates at the evdev/uinput level, same as keyd and kanata.

## How it works

1. You press `e` -- it appears **immediately** (zero latency)
2. You hold for 300ms -- repeat stops, popup shows accented variants
3. You release the key -- popup stays open (macOS-style, configurable)
4. You press `2` -- backspace + `é` is emitted. Popup closes
5. Or you press ESC -- popup closes, original `e` stays

**Fast typing is never affected.** If you press another key within 300ms, the hold timer cancels instantly. Only accent-eligible keys (a, c, e, i, n, o, s, u, y) trigger detection.

## Architecture

```
+----------+   evdev grab   +----------+   uinput   +---------+
| Keyboard | -------------> |  accentd | ---------> | Display |
|/dev/input|                |  (daemon)|            | Server  |
+----------+                +----+-----+            +---------+
                                 |
                            Unix socket
                            (JSON-lines)
                                 |
                         +-------+--------+
                         | accentd-popup  |
                         | (GTK4 + layer- |
                         |     shell)     |
                         +----------------+
```

Three binaries:

- **accentd** -- system daemon that grabs keyboards via evdev, runs a state machine, and emits keystrokes via uinput
- **accentd-popup** -- GTK4 user service that displays the accent selection overlay
- **accentctl** -- CLI for controlling the daemon (toggle, set locale, status)

Communication is via a Unix socket with JSON-lines messages.

## Install

### Arch Linux (AUR)

```bash
paru -S accentd   # or yay -S accentd

# Then enable (both are user services, no root)
systemctl --user enable --now accentd accentd-popup
```

Your user must be in the `input` group (log out and back in after):

```bash
sudo usermod -aG input $USER
```

### Ubuntu / Debian

Install the build dependencies (Ubuntu 24.04+; on older releases the
layer-shell development package is `libgtk-layer-shell-dev`):

```bash
sudo apt update
sudo apt install -y build-essential pkg-config libgtk-4-dev libgtk4-layer-shell-dev
```

Install the Rust toolchain if you do not have it:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Then follow [From source](#from-source-any-distro) to build and install. The
resulting `accentd` runs as a user service, not as root.

### From source (any distro)

Requirements: Rust toolchain, GTK4, gtk4-layer-shell

The daemon runs as a user service, not as root, so it never needs system-wide
privileges at runtime. It does need read access to `/dev/input/event*` and
write access to `/dev/uinput`, which the `input` group provides:

```bash
sudo usermod -aG input "$USER"   # log out and back in afterwards
```

Build and install:

```bash
cargo build --release

# Install binaries
sudo install -Dm755 target/release/accentd /usr/bin/accentd
sudo install -Dm755 target/release/accentd-popup /usr/bin/accentd-popup
sudo install -Dm755 target/release/accentctl /usr/bin/accentctl

# Install locale data (optional; the same locales are compiled in)
sudo install -dm755 /usr/share/accentd/locales
sudo install -Dm644 data/locales/*.toml /usr/share/accentd/locales/

# Install user services
install -Dm644 dist/accentd.service ~/.config/systemd/user/accentd.service
install -Dm644 dist/accentd-popup.service ~/.config/systemd/user/accentd-popup.service

# Install udev rule (allows access to /dev/uinput)
sudo install -Dm644 dist/70-accentd.rules /usr/lib/udev/rules.d/70-accentd.rules
sudo udevadm control --reload-rules

# Enable (both run in your user session)
systemctl --user enable --now accentd accentd-popup
```

The daemon needs access to `/dev/input/event*` and `/dev/uinput`. The included udev rule handles this. Your user must be in the `input` group (log out and back in after `usermod`).

### macOS accent order

The active locale decides the variants and their order, so
`accentctl set-locale de` gives the German list (where `ä`, `ö` and `ü` are all
`1`). The macOS U.S./ABC list ships as the `macos` locale:
`accentctl set-locale macos`.

Build with the `macos-order` feature to make `macos` the default when no config
file exists:

```bash
cargo build --release --features accentd/macos-order
```

## Usage

```bash
# Check status
accentctl status

# Change locale
accentctl set-locale fr

# Toggle on/off (bind this to a WM keybinding)
accentctl toggle

# Disable / enable
accentctl disable
accentctl enable
```

## Configuration

`~/.config/accentd/config.toml`

```toml
[general]
threshold_ms = 300   # hold time before popup appears
enabled = true

[popup]
font_size = 24
timeout_ms = 5000    # auto-dismiss popup after 5s
keep_open = true     # macOS-style: popup stays after key release

[locale]
active = "it"
```

## Locales

Language locales: **it** (Italian), **es** (Spanish), **fr** (French), **de** (German), **pt** (Portuguese). These use compact per-letter lists, ordered so the language's accents line up (for German, `ä`, `ö` and `ü` are all `1`).

Two more built-in locales:

- **macos** -- the U.S./ABC lists from Apple's `PressAndHold.plist`, so the numbers match a Mac. Compact, but not accent-aligned (`ä` is `4`, `ö`/`ü` are `2`).
- **unicode** -- every precomposed Latin letter, generated from Unicode composition. The accent sits in a fixed slot across all letters, so the same accent is always the same number. Missing accents leave holes: `c` shows `2:ć 3:ĉ 9:č 0:ç`. Slots are `1` grave, `2` acute, `3` circumflex, `4` tilde, `5` diaeresis, `6` ring, `7` macron, `8` breve, `9` caron, `0` cedilla. The first nine use the number row, the tenth is `0`.

Switch with `accentctl set-locale <name>`, for example `accentctl set-locale de` or `accentctl set-locale unicode`.

Custom locales can be added as TOML files in `~/.config/accentd/locales/` or `/usr/share/accentd/locales/`:

```toml
# ~/.config/accentd/locales/custom.toml
a = ["ā", "ă", "ą"]
e = ["ē", "ĕ", "ę"]
```

Then: `accentctl set-locale custom`

## Popup display

| Environment | Method |
|---|---|
| Sway, Hyprland, KDE Wayland | gtk4-layer-shell overlay |
| GNOME Wayland | Undecorated GTK4 window (degraded positioning) |
| X11 | Undecorated GTK4 window |
| TTY / headless | No popup (number selection still works blind) |

## Known limitations

- **QWERTY only.** Keycodes assume a QWERTY physical layout. Dvorak, AZERTY, Colemak will map to the wrong letters.
- **Ctrl+Shift+U input method.** Accent emission works in GTK and Qt apps. May fail in Electron apps, some terminal emulators, and other toolkits that don't support this input method.
- **GNOME Wayland.** The popup uses wlr-layer-shell for overlay positioning. GNOME doesn't support this protocol, so the popup falls back to a regular window with degraded positioning.

## Resilience

- **Daemon crashes** -- evdev grab is released automatically (fd close), keyboard returns to normal
- **Popup crashes** -- daemon continues working, popup restarts via systemd
- **Multiple keyboards** -- independent state machine per device
- **Panic key combo** -- press Backspace, Escape, Enter in quick succession to force-exit the daemon and release the keyboard grab. Safety escape hatch if the daemon hangs.

## Security

accentd uses EVIOCGRAB to exclusively grab keyboard input devices, the same mechanism used by keyd, kanata, and other input remapping daemons. This means the daemon has full access to all keyboard input while running.

The daemon:
- Only reads key events and emits accented characters through uinput
- Runs with systemd security hardening (NoNewPrivileges, ProtectHome, ProtectSystem=strict)
- Communicates with the popup over a local Unix socket with filesystem permissions

This is the same trust model as keyd and kanata.

## License

MIT
