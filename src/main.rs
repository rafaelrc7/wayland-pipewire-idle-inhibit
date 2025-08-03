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

use std::{
    error::Error,
    process::ExitCode,
    sync::{
        atomic::{self, AtomicBool},
        Arc,
    },
};

mod inhibit_idle_state;
use inhibit_idle_state::{InhibitIdleState, InhibitIdleStateEvent};

mod pipewire_connection;
use message_queue::MessageQueueReceiver;
use pipewire_connection::{PWEvent, PWMsg, PWThread};

mod idle_inhibitor;
use idle_inhibitor::{
    dbus::DbusIdleInhibitor,
    dry::DryRunIdleInhibitor,
    wayland::{WaylandEventQueue, WaylandIdleInhibitor},
    IdleInhibitor,
};

mod settings;
use settings::Settings;

mod message_queue;

use nix::{errno::Errno, sys::epoll::*};

#[repr(u64)]
enum MessageQueueType {
    Unknown,
    Wayland,
    Main,
}

impl From<u64> for MessageQueueType {
    fn from(value: u64) -> Self {
        match value {
            value if value == Self::Wayland as u64 => Self::Wayland,
            value if value == Self::Main as u64 => Self::Main,
            _ => Self::Unknown,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Msg {
    PWEvent(PWEvent),
    InhibitIdleStateEvent(InhibitIdleStateEvent),
}

impl Msg {
    fn handle(
        &self,
        pw_thread: &PWThread,
        inhibit_idle_state_manager: &mut InhibitIdleState<Msg>,
        idle_inhibitor: &mut dyn IdleInhibitor,
    ) -> Result<(), Box<dyn Error>> {
        match self {
            Msg::PWEvent(pw_event) => match pw_event {
                PWEvent::GraphUpdated => {
                    pw_thread.send(PWMsg::GraphUpdated)?;
                }

                PWEvent::InhibitIdleState(inhibit_idle_state) => {
                    inhibit_idle_state_manager.set_is_idle_inhibited(*inhibit_idle_state);
                }
            },

            Msg::InhibitIdleStateEvent(inhibit_idle_state_event) => {
                match inhibit_idle_state_event {
                    InhibitIdleStateEvent::InhibitIdle(inhibit_idle_state) => {
                        if *inhibit_idle_state {
                            idle_inhibitor.inhibit()?;
                        } else {
                            idle_inhibitor.uninhibit()?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
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

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            log::error!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let settings = Settings::new()?;

    simplelog::TermLogger::init(
        settings.get_verbosity(),
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )?;

    let epoll = Epoll::new(EpollCreateFlags::empty())?;
    let (mq, mq_receiver) =
        message_queue::message_queue::<Msg>(&epoll, MessageQueueType::Main as u64)?;

    let pw_thread = PWThread::new(
        mq.clone(),
        settings.get_sink_whitelist().to_vec(),
        settings.get_node_blacklist().to_vec(),
    );

    let inhibit_idle_state_manager: InhibitIdleState<Msg> =
        InhibitIdleState::new(settings.get_media_minimum_duration(), mq.clone());

    let term = Arc::new(AtomicBool::new(false));
    for sig in signal_hook::consts::TERM_SIGNALS {
        signal_hook::flag::register(*sig, Arc::clone(&term))?;
    }

    match settings.get_idle_inhibitor() {
        settings::IdleInhibitor::DBus => {
            let idle_inhibitor = Box::new(DbusIdleInhibitor::new()?);
            non_wayland_main_loop(
                idle_inhibitor,
                term,
                epoll,
                mq_receiver,
                &pw_thread,
                inhibit_idle_state_manager,
            )?;
        }
        settings::IdleInhibitor::DryRun => {
            let idle_inhibitor = Box::<DryRunIdleInhibitor>::default();
            non_wayland_main_loop(
                idle_inhibitor,
                term,
                epoll,
                mq_receiver,
                &pw_thread,
                inhibit_idle_state_manager,
            )?;
        }
        settings::IdleInhibitor::Wayland => {
            let (idle_inhibitor, event_queue) = WaylandIdleInhibitor::new()?;
            wayland_main_loop(
                idle_inhibitor,
                event_queue,
                term,
                epoll,
                mq_receiver,
                &pw_thread,
                inhibit_idle_state_manager,
            )?;
        }
    };

    pw_thread.send(PWMsg::Terminate)?;
    pw_thread.join()?;

    Ok(())
}

fn wayland_main_loop(
    mut wayland_idle_inhibitor: WaylandIdleInhibitor,
    mut wayland_event_queue: WaylandEventQueue,
    term: Arc<AtomicBool>,
    epoll: Epoll,
    mq_receiver: MessageQueueReceiver<Msg>,
    pw_thread: &PWThread,
    mut inhibit_idle_state_manager: InhibitIdleState<Msg>,
) -> Result<(), Box<dyn Error>> {
    while !term.load(atomic::Ordering::Relaxed) {
        wayland_event_queue.flush()?;
        let wayland_read_guard =
            if let Some(wayland_read_guard) = wayland_event_queue.prepare_read() {
                wayland_read_guard
            } else {
                wayland_event_queue.dispatch_pending(&mut wayland_idle_inhibitor)?;
                wayland_event_queue.prepare_read().ok_or(
                    "Unknown error when trying to get a read lock on the Wayland Event Queue",
                )?
            };

        epoll.add(
            wayland_read_guard.connection_fd(),
            EpollEvent::new(EpollFlags::EPOLLIN, MessageQueueType::Wayland as u64),
        )?;

        let mut events = [EpollEvent::empty()];
        let event = match epoll.wait(&mut events, EpollTimeout::NONE) {
            Ok(_) => events[0],
            Err(Errno::EINTR) => continue,
            Err(err) => Err(err)?,
        };

        match event.data().into() {
            MessageQueueType::Main => {
                epoll.delete(wayland_read_guard.connection_fd())?;
                std::mem::drop(wayland_read_guard);
                mq_receiver.recv()?.handle(
                    pw_thread,
                    &mut inhibit_idle_state_manager,
                    &mut wayland_idle_inhibitor,
                )?;
            }

            MessageQueueType::Wayland => {
                epoll.delete(wayland_read_guard.connection_fd())?;
                if wayland_read_guard.read().is_ok() {
                    wayland_event_queue.dispatch_pending(&mut wayland_idle_inhibitor)?;
                }
            }

            MessageQueueType::Unknown => log::error!(target: "main", "Unknown event queue"),
        }
    }
    Ok(())
}

fn non_wayland_main_loop(
    mut idle_inhibitor: Box<dyn IdleInhibitor>,
    term: Arc<AtomicBool>,
    epoll: Epoll,
    mq_receiver: MessageQueueReceiver<Msg>,
    pw_thread: &PWThread,
    mut inhibit_idle_state_manager: InhibitIdleState<Msg>,
) -> Result<(), Box<dyn Error>> {
    while !term.load(atomic::Ordering::Relaxed) {
        let mut events = [EpollEvent::empty()];
        let event = match epoll.wait(&mut events, EpollTimeout::NONE) {
            Ok(_) => events[0],
            Err(Errno::EINTR) => continue,
            Err(err) => Err(err)?,
        };

        match event.data().into() {
            MessageQueueType::Main => match mq_receiver.recv()? {
                Msg::PWEvent(pw_event) => match pw_event {
                    PWEvent::GraphUpdated => {
                        pw_thread.send(PWMsg::GraphUpdated)?;
                    }

                    PWEvent::InhibitIdleState(inhibit_idle_state) => {
                        inhibit_idle_state_manager.set_is_idle_inhibited(inhibit_idle_state);
                    }
                },

                Msg::InhibitIdleStateEvent(inhibit_idle_state_event) => {
                    match inhibit_idle_state_event {
                        InhibitIdleStateEvent::InhibitIdle(inhibit_idle_state) => {
                            if inhibit_idle_state {
                                idle_inhibitor.inhibit()?;
                            } else {
                                idle_inhibitor.uninhibit()?;
                            }
                        }
                    }
                }
            },

            MessageQueueType::Wayland => {}

            MessageQueueType::Unknown => log::error!(target: "main", "Unknown event queue"),
        }
    }
    Ok(())
}
