# Wayland PipeWire Idle Inhibit

Suspends automatic idling of Wayland compositors when media is being played
through Pipewire.

Depends on the Wayland experimental protocol
[idle-inhibit-unstable-v1](https://wayland.app/protocols/idle-inhibit-unstable-v1)
and [PipeWire](https://www.pipewire.org/).

## Building

### Nix

```sh
git clone https://github.com/rafaelrc7/wayland-pipewire-idle-inhibit
cd wayland-pipewire-idle-inhibit
nix build
```

```sh
nix build github:rafaelrc7/wayland-pipewire-idle-inhibit
```

### Cargo

```sh
git clone https://github.com/rafaelrc7/wayland-pipewire-idle-inhibit
cd wayland-pipewire-idle-inhibit
cargo build
```

## Installing

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
      { name = "Music Player Daemon"; }
    ];
  };
};
```

#### Using the overlay

```nix
inputs.wayland-pipewire-idle-inhibit.overlays.default
```

#### Using the package

```nix
inputs.wayland-pipewire-idle-inhibit.packages.default
```

### Cargo

```sh
git clone https://github.com/rafaelrc7/wayland-pipewire-idle-inhibit
cd wayland-pipewire-idle-inhibit
cargo install
```

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
  -c, --config <PATH>
          Path to config file
  -h, --help
          Print help
  -V, --version
          Print version
```

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
