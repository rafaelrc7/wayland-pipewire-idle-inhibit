// Copyright (C) 2023  Rafael Carvalho <contact@rafaelrc.com>

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-only

use std::thread;

mod pipewire_connection;
mod wayland_connection;
use wayland_connection::WaylandConnection;

use signal_hook::{
    consts::{SIGINT, SIGQUIT, SIGTERM},
    iterator::Signals,
};

fn main() {
    env_logger::init();

    let mut wayland_connection = WaylandConnection::new();
    let (pw_thread, pw_thread_terminate) = pipewire_connection::PWThread::new();

    let mut signals =
        Signals::new(&[SIGINT, SIGQUIT, SIGTERM]).expect("Failed to create signal listener");
    let signal_thread = thread::spawn(move || {
        for _sig in signals.wait() {
            pw_thread_terminate.send();
        }
    });

    pw_thread.run(move |inhibit_idle| wayland_connection.set_inhibit_idle(inhibit_idle));
    signal_thread.join().unwrap();
}
