// Copyright (C) 2023-2025  Rafael Carvalho <contact@rafaelrc.com>

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as published by
// the Free Software Foundation.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

// SPDX-License-Identifier: GPL-3.0-only

//! Inhibit idle in Wayland compositors when audio is being played through PipeWire, with highly
//! customisable options

use std::thread;

use signal_hook::{consts::TERM_SIGNALS, iterator::Signals};

mod inhibit_idle_state;
use inhibit_idle_state::{InhibitIdleState, InhibitIdleStateEvent};

mod pipewire_connection;
use pipewire_connection::{PWEvent, PWMsg, PWThread};

mod idle_inhibitor;
use idle_inhibitor::{
    dbus::DbusIdleInhibitor, dry::DryRunIdleInhibitor, wayland::WaylandIdleInhibitor, IdleInhibitor,
};

mod settings;
use settings::Settings;

mod message_queue;

use nix::{errno::Errno, sys::epoll::*};

#[repr(u64)]
enum MessageQueueType {
    WaylandQueue,
    MainQueue,
}

impl TryFrom<u64> for MessageQueueType {
    type Error = ();

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            value if value == Self::WaylandQueue as u64 => Ok(Self::WaylandQueue),
            value if value == Self::MainQueue as u64 => Ok(Self::MainQueue),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Msg {
    PWEvent(PWEvent),
    InhibitIdleStateEvent(InhibitIdleStateEvent),
    Terminate,
}

impl From<PWEvent> for Msg {
    fn from(value: PWEvent) -> Self {
        Msg::PWEvent(value)
    }
}

impl From<InhibitIdleStateEvent> for Msg {
    fn from(value: InhibitIdleStateEvent) -> Self {
        Msg::InhibitIdleStateEvent(value)
    }
}

fn main() {
    let settings = match Settings::new() {
        Ok(settings) => settings,
        Err(error) => panic!("{}", error),
    };

    simplelog::TermLogger::init(
        settings.get_verbosity(),
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
    .unwrap();

    let epoll = Epoll::new(EpollCreateFlags::empty()).unwrap();
    let (mq, mq_receiver) =
        message_queue::message_queue::<Msg>(&epoll, MessageQueueType::MainQueue as u64).unwrap();

    let mut signals = Signals::new(TERM_SIGNALS).expect("Failed to create signal listener");
    let signal_thread = thread::spawn({
        let mq = mq.clone();
        move || {
            for _sig in signals.wait() {
                mq.send(Msg::Terminate).unwrap();
            }
        }
    });

    let pw_thread = PWThread::new(
        mq.clone(),
        settings.get_sink_whitelist().to_vec(),
        settings.get_node_blacklist().to_vec(),
    );

    let mut idle_inhibitor: Box<dyn IdleInhibitor> = match settings.get_idle_inhibitor() {
        settings::IdleInhibitor::DBus => match DbusIdleInhibitor::new() {
            Ok(dbus_idle_inhibitor) => Box::new(dbus_idle_inhibitor),
            Err(error) => panic!("{}", error),
        },
        settings::IdleInhibitor::Wayland => match WaylandIdleInhibitor::new() {
            Ok(wayland_idle_inhibitor) => Box::new(wayland_idle_inhibitor),
            Err(error) => panic!("{}", error),
        },
        settings::IdleInhibitor::DryRun => Box::<DryRunIdleInhibitor>::default(),
    };

    let mut inhibit_idle_state_manager: InhibitIdleState<Msg> =
        InhibitIdleState::new(settings.get_media_minimum_duration(), mq.clone());

    loop {
        let wayland_read_guard = idle_inhibitor.wayland_queue_read_guard().unwrap();
        if let Some(wayland_read_guard) = &wayland_read_guard {
            epoll
                .add(
                    wayland_read_guard.connection_fd(),
                    EpollEvent::new(EpollFlags::EPOLLIN, MessageQueueType::WaylandQueue as u64),
                )
                .unwrap();
        }
        let event = epoll_wait(&epoll).unwrap();
        match event.data().try_into() {
            Ok(MessageQueueType::MainQueue) => {
                if let Some(wayland_read_guard) = wayland_read_guard {
                    epoll.delete(wayland_read_guard.connection_fd()).unwrap();
                    std::mem::drop(wayland_read_guard);
                }
                match mq_receiver.recv().unwrap() {
                    Msg::PWEvent(pw_event) => match pw_event {
                        PWEvent::GraphUpdated => {
                            pw_thread.send(PWMsg::GraphUpdated).unwrap();
                        }

                        PWEvent::InhibitIdleState(inhibit_idle_state) => {
                            inhibit_idle_state_manager.set_is_idle_inhibited(inhibit_idle_state);
                        }
                    },

                    Msg::InhibitIdleStateEvent(inhibit_idle_state_event) => {
                        match inhibit_idle_state_event {
                            InhibitIdleStateEvent::InhibitIdle(inhibit_idle_state) => {
                                if let Err(error) = if inhibit_idle_state {
                                    idle_inhibitor.inhibit()
                                } else {
                                    idle_inhibitor.uninhibit()
                                } {
                                    panic!("{}", error);
                                }
                            }
                        }
                    }

                    Msg::Terminate => {
                        pw_thread.send(PWMsg::Terminate).unwrap();
                        break;
                    }
                }
            }

            Ok(MessageQueueType::WaylandQueue) => {
                if let Some(wayland_read_guard) = wayland_read_guard {
                    epoll.delete(wayland_read_guard.connection_fd()).unwrap();
                    wayland_read_guard.read().unwrap();
                    idle_inhibitor.wayland_dispatch_pending().unwrap();
                }
            }

            _ => log::error!(target: "main", "Unknown event queue"),
        }
    }

    pw_thread.join().unwrap();
    signal_thread.join().unwrap();
}

fn epoll_wait(epoll: &Epoll) -> Result<EpollEvent, Errno> {
    let mut events = [EpollEvent::empty()];
    match epoll.wait(&mut events, EpollTimeout::NONE) {
        Ok(_) => Ok(events[0]),
        Err(Errno::EINTR) => epoll_wait(epoll),
        Err(err) => Err(err),
    }
}
