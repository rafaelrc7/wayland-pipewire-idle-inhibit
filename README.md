# Wayland PipeWire Idle Inhibit

## Description

Suspends automatic idling when media is being played through Pipewire.

For detecting media being played, it depends on [PipeWire](https://www.pipewire.org/).

For inhibiting idle, it depends, either on:

- Wayland compositors implementing the experimental protocol
  [idle-inhibit-unstable-v1](https://wayland.app/protocols/idle-inhibit-unstable-v1)
- Daemons implementing the D-Bus
  [org.freedesktop.ScreenSaver](https://specifications.freedesktop.org/idle-inhibit-spec/latest/re01.html)
  service

### Main features

- Inhibit idle when any app plays audio through PipeWire
- Customisable minimum media duration to inhibit idle (Useful for keeping
  notifications from inhibiting idle)
- Customisable list of client filters (Useful for ignoring certain programs,
  such as background music)
- Support for idle inhibiting through Wayland compositors and dbus services

Feedback and contributions are welcome!

## Tested on

- Sway: works fine with the default wayland idle inhibitor
- Plasma: while in theory it implements the `idle-inhibit-unstable-v1` protocol
  it seems to be broken. Works fine using the dbus idle inhibitor.

Should work fine with any compositor that implements `idle-inhibit-unstable-v1`
or any compositor/DE that offers the `org.freedesktop.ScreenSaver` service.

## Availability

- [AUR](#aur)
- [Cargo](#cargo-1)
- [Nix Flake](#nix-flake-recommended)

## Usage

```
Usage: wayland-pipewire-idle-inhibit [OPTIONS]

Options:
  -d, --media-minimum-duration <SECONDS>
          Minimum media duration to inhibit idle
  -v, --verbosity <VERBOSITY>
          Log verbosity [possible values: OFF, ERROR, WARN, INFO, DEBUG, TRACE]
  -q, --quiet
          Disables logging completely
  -b, --dbus
          Enable DBus (org.freedesktop.ScreenSaver) idle inhibitor
  -B, --no-dbus
          Disables DBus idle inhibitor
  -w, --wayland
          Enable Wayland idle inhibitor (Enabled by default)
  -W, --no-wayland
          Disables Wayland idle inhibitor
  -n, --dry-run
          Only logs (at INFO level) about idle inhibitor state changes
  -c, --config <PATH>
          Path to config file
  -h, --help
          Print help
  -V, --version
          Print version
```

## Building

### Cargo

```sh
git clone https://github.com/rafaelrc7/wayland-pipewire-idle-inhibit
cd wayland-pipewire-idle-inhibit
cargo build
```

### Nix

```sh
git clone https://github.com/rafaelrc7/wayland-pipewire-idle-inhibit
cd wayland-pipewire-idle-inhibit
nix build
```

```sh
nix build github:rafaelrc7/wayland-pipewire-idle-inhibit
```

## Installing

### Cargo

```sh
cargo install wayland-pipewire-idle-inhibit
```

See [Running](#running) for instructions on how to run the program.

### AUR

This package is available in the Arch User Repository:
[wayland-pipewire-idle-inhibit](https://aur.archlinux.org/packages/wayland-pipewire-idle-inhibit)

Install it using your AUR helper of choice.

The package includes the binary and the default systemd service unit file, that
may be enabled and ran with:

```sh
systemctl --user enable wayland-pipewire-idle-inhibit.service --now
```

See [Running](#running) for further instructions on how to run the program.

### Nix Flake (recommended)

Add the following snippet to your flake inputs:

```nix
wayland-pipewire-idle-inhibit = {
  url = "github:rafaelrc7/wayland-pipewire-idle-inhibit";
  inputs.nixpkgs.follows = "nixpkgs";
};
```

From this point you have many options:

#### Using the Home Manager module (recommended)

Add the following to your home-manager imports:

```nix
inputs.wayland-pipewire-idle-inhibit.homeModules.default
```

And then you may use the option to set it up, for example:

```nix
services.wayland-pipewire-idle-inhibit = {
  enable = true;
  systemdTarget = "sway-session.target";
  settings = {
    verbosity = "INFO";
    media_minimum_duration = 10;
    sink_whitelist = [
      { name = "Starship/Matisse HD Audio Controller Analog Stereo"; }
    ];
    node_blacklist = [
      { name = "spotify"; }
      { app_name = "Music Player Daemon"; }
    ];
  };
};
```

This method will install the program and setup a user systemd service.

#### Using the overlay

```nix
inputs.wayland-pipewire-idle-inhibit.overlays.default
```

By adding the overlay, you may then install the `wayland-pipewire-idle-inhibit`
package.

See [Running](#running) for instructions on how to run the program.

#### Using the package

```nix
inputs.wayland-pipewire-idle-inhibit.packages.default
```

See [Running](#running) for instructions on how to run the program.

## Running

### Compositor

Then you may run it in your Sway config, or equivalent for your Wayland
compositor:

```
exec wayland-pipewire-idle-inhibit
```

### systemd

Another option is to setup a systemd user service. See
[wayland-pipewire-idle-inhibit.service](wayland-pipewire-idle-inhibit.service)
for a model. You may customise it by, for example, adding CLI args to
`ExecStart` or changing the `WantedBy` target to, for instance,
`sway-session.target`.

- Copy the example service file to `~/.config/systemd/user/` and edit it to
  your liking
- Run `systemctl --user daemon-reload`
- Run `systemctl --user enable wayland-pipewire-idle-inhibit.service --now`

## Config

Most settings may be defined either via CLI arguments (run the program with the
`--help` for more information) or config file. CLI arguments have priority over
the config file. The default config file path is
`~/.config/wayland-pipewire-idle-inhibit/config.toml`, but other path may be
set using `--config <PATH>`.

`~/.config/wayland-pipewire-idle-inhibit/config.toml` with the default options

```toml
verbosity = "WARN"
media_minimum_duration = 5
sink_whitelist = [ ]
node_blacklist = [ ]
```

### Sink Whitelist

You may set a list of Sink filters to be considered by the program. If the Sink
matches any of the filters, it will be used.

#### Supported fields

- `name`: Regex

#### Example

```toml
[[sink_whitelist]]
name = "Sink 1 name"

[[sink_whitelist]]
name = "Another Sink"
```

### Node (Client) Blacklist

You may set a list of Node filters to be ignored and not inhibit idle even when
playing media. If the node matches any of the filters, it will be ignored.

#### Supported fields

- `name`: Regex. This name is the same used by Helvum for the node.
- `app_name`: Regex
- `media_class`: Regex
- `media_role`: Regex
- `media_software`: Regex

#### Example

```toml
[[node_blacklist]]
name = "[Ff]irefox"
```

## Thanks

- [Misterio77](https://github.com/Misterio77/) For help with the creation of
  the home-manager module.

This project was inspired by

- [SwayAudioIdleInhibit](https://github.com/ErikReider/SwayAudioIdleInhibit)
- [Helvum](https://gitlab.freedesktop.org/pipewire/helvum)

## Licence and Credits

This project is licensed under the terms of the GPL3 licence. See
[LICENCE](LICENCE) for more information.

Parts of the code of the PipeWire connection were greatly inspired by
[Helvum](https://gitlab.freedesktop.org/pipewire/helvum), which is also
licensed under the terms of the GPL3 licence.
