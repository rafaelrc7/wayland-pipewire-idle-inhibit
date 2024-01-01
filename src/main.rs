// Copyright (C) 2023-2024  Rafael Carvalho <contact@rafaelrc.com>

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

use std::{sync::mpsc, thread};

mod inhibit_idle_state;
use chrono::Duration;
use inhibit_idle_state::{InhibitIdleState, InhibitIdleStateEvent};

mod pipewire_connection;
use pipewire_connection::{PWEvent, PWMsg, PWThread};

mod wayland_idle_inhibitor;
use wayland_idle_inhibitor::WaylandIdleInhibitor;

use signal_hook::{
    consts::{SIGINT, SIGQUIT, SIGTERM},
    iterator::Signals,
};

#[derive(Debug)]
enum Msg {
    PWGraphUpdated,
    PWInhibitIdleState(bool),
    IIEInhibitIdleState(bool),
    Terminate,
}

impl From<PWEvent> for Msg {
    fn from(value: PWEvent) -> Self {
        match value {
            PWEvent::GraphUpdated => Msg::PWGraphUpdated,
            PWEvent::InhibitIdleState(inhibit_idle_state) => {
                Msg::PWInhibitIdleState(inhibit_idle_state)
            }
        }
    }
}

impl From<InhibitIdleStateEvent> for Msg {
    fn from(value: InhibitIdleStateEvent) -> Self {
        match value {
            InhibitIdleStateEvent::InhibitIdle(inhibit_idle_state) => {
                Msg::IIEInhibitIdleState(inhibit_idle_state)
            }
        }
    }
}

fn main() {
    env_logger::init();

    let (event_queue_sender, event_queue) = mpsc::channel::<Msg>();

    let mut signals =
        Signals::new([SIGINT, SIGQUIT, SIGTERM]).expect("Failed to create signal listener");
    let signal_thread = thread::spawn({
        let event_queue_sender = event_queue_sender.clone();
        move || {
            for _sig in signals.wait() {
                event_queue_sender.send(Msg::Terminate).unwrap();
            }
        }
    });

    let pw_thread = PWThread::new(event_queue_sender.clone());
    let mut wayland_idle_inhibitor = WaylandIdleInhibitor::new();
    let mut inhibit_idle_state_manager: InhibitIdleState<Msg> =
        InhibitIdleState::new(Some(Duration::seconds(5)), event_queue_sender);

    loop {
        match event_queue.recv().unwrap() {
            Msg::PWGraphUpdated => {
                pw_thread.send(PWMsg::GraphUpdated).unwrap();
            }
            Msg::PWInhibitIdleState(inhibit_idle_state) => {
                inhibit_idle_state_manager.set_is_idle_inhibited(inhibit_idle_state);
            }
            Msg::IIEInhibitIdleState(inhibit_idle_state) => {
                wayland_idle_inhibitor.set_inhibit_idle(inhibit_idle_state);
            }
            Msg::Terminate => {
                pw_thread.send(PWMsg::Terminate).unwrap();
                break;
            }
        }
    }

    pw_thread.join().unwrap();
    signal_thread.join().unwrap();
}
