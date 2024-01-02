# Wayland PipeWire Idle Inhibit

Suspends automatic idling of Wayland compositors when media is being played
through Pipewire.

Depends on the experimental protocol
[idle-inhibit-unstable-v1](https://wayland.app/protocols/idle-inhibit-unstable-v1)
and [PipeWire](https://www.pipewire.org/).

## Roadmap

This is the first usable version of the project. You just need to run the built program. However, many improvements are planned for the short futures such as:

- [x] Refactoring of the PipeWire connection code.
- [x] Customisation of the minimum audio duration to trigger idle inhibition.
- [x] Sink whitelist.
- [ ] Software blacklist.
- [ ] Configuration File.

## Building

### Nix

`nix build`

### Cargo

`cargo build`

## Usage

run
`wayland-pipewire-idle-inhibit --help`
for a list of options.

## Thanks

This project was inspired by

- [SwayAudioIdleInhibit](https://github.com/ErikReider/SwayAudioIdleInhibit)
- [Helvum](https://gitlab.freedesktop.org/pipewire/helvum)

## Licence and Credits

This project is licensed under the terms of the GPL3 licence. See [LICENCE](LICENCE) for
more information.

Parts of the code of the PipeWire connection were greatly inspired by
[Helvum](https://gitlab.freedesktop.org/pipewire/helvum), which is also
licensed under the terms of the GPL3 licence.
