# accentd

Press-and-hold accent character popup for Linux, like macOS.

Hold a letter key (e.g. `e`) for 300ms and a popup appears with numbered accented variants:

```
1:è  2:é  3:ê  4:ë
```

Press a number to replace the letter. Release or press ESC to cancel.

Works on any desktop environment and display server: GNOME, KDE, Sway, Hyprland, X11, Wayland. Operates at the evdev/uinput level, same as keyd and kanata.

## How it works

1. You press `e` -- it appears **immediately** (zero latency)
2. You hold for 300ms -- repeat stops, popup shows accented variants
3. You press `2` -- backspace + `é` is emitted. Popup closes
4. Or you release/ESC -- popup closes, original `e` stays

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

### Arch Linux

```bash
cd ~/src/accentd
makepkg -si -p dist/PKGBUILD

# Then enable
sudo systemctl enable --now accentd
systemctl --user enable --now accentd-popup
```

### From source (any distro)

Requirements: Rust toolchain, GTK4, gtk4-layer-shell

```bash
cargo build --release

# Install binaries
sudo install -Dm755 target/release/accentd /usr/bin/accentd
sudo install -Dm755 target/release/accentd-popup /usr/bin/accentd-popup
sudo install -Dm755 target/release/accentctl /usr/bin/accentctl

# Install locale data
sudo install -dm755 /usr/share/accentd/locales
sudo install -Dm644 data/locales/*.toml /usr/share/accentd/locales/

# Install systemd services
sudo install -Dm644 dist/accentd.service /usr/lib/systemd/system/accentd.service
install -Dm644 dist/accentd-popup.service ~/.config/systemd/user/accentd-popup.service

# Install udev rule (allows access to /dev/uinput)
sudo install -Dm644 dist/70-accentd.rules /usr/lib/udev/rules.d/70-accentd.rules
sudo udevadm control --reload-rules

# Enable
sudo systemctl enable --now accentd
systemctl --user enable --now accentd-popup
```

### Permissions

The daemon needs access to `/dev/input/event*` (read keyboards) and `/dev/uinput` (emit keystrokes). The included udev rule and systemd service handle this. Your user must be in the `input` group:

```bash
sudo usermod -aG input $USER
# Log out and back in
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

[locale]
active = "it"
```

## Locales

Built-in: **it** (Italian), **es** (Spanish), **fr** (French), **de** (German), **pt** (Portuguese)

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
