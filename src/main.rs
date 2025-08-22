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
    io::{self, Write},
    panic,
    process::ExitCode,
    sync::{
        Arc,
        atomic::{self, AtomicBool},
    },
};

mod dbus_service;
mod idle_inhibitor;
mod inhibit_idle_state;
mod message_queue;
mod pipewire_connection;
mod settings;

use idle_inhibitor::{
    dbus::DbusIdleInhibitor,
    dry::DryRunIdleInhibitor,
    wayland::{WaylandEventQueue, WaylandIdleInhibitor},
    IdleInhibitor,
};
use inhibit_idle_state::{InhibitIdleState, InhibitIdleStateEvent};
use message_queue::MessageQueueReceiver;
use nix::{errno::Errno, sys::epoll::*};
use pipewire_connection::{PWEvent, PWMsg, PWThread};
use settings::Settings;

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

#[derive(Clone, Debug)]
pub enum Msg {
    PWEvent(PWEvent),
    InhibitIdleStateEvent(InhibitIdleStateEvent),
    ToggleManual,
}

fn print_waybar_status(inhibited: bool) {
    let icon = if inhibited { "☕" } else { "⌚" };
    let text = if inhibited {
        "Idle Inhibited"
    } else {
        "Idling"
    };

    println!("{{\"text\":\"{}\", \"tooltip\":\"{}\"}}", icon, text);
    io::stdout().flush().unwrap();
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
                    inhibit_idle_state_manager.set_is_audio_inhibited(*inhibit_idle_state);
                }

                PWEvent::ThreadPanic(err) => {
                    if let Some(err) = err {
                        Err(format!("Fatal PipeWire Error: {err}"))?;
                    } else {
                        Err("Fatal PipeWire Error!")?;
                    }
                }
            },

            Msg::InhibitIdleStateEvent(inhibit_idle_state_event) => {
                match inhibit_idle_state_event {
                    InhibitIdleStateEvent::InhibitIdle(inhibit_idle_state) => {
                        idle_inhibitor.set_inhibit_idle(*inhibit_idle_state)?;
                        print_waybar_status(*inhibit_idle_state);
                    }
                    InhibitIdleStateEvent::AudioInhibitTimerFired => {
                        inhibit_idle_state_manager.set_is_inhibited_from_timer();
                    }
                }
            }
            Msg::ToggleManual => {
                inhibit_idle_state_manager.toggle_manual_inhibit();
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

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(())
         => ExitCode::SUCCESS,
        Err(error) => {
            log::error!("{error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
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

    tokio::spawn(dbus_service::start_dbus_service(mq.clone()));

    panic::set_hook(Box::new({
        let mq = mq.clone();
        move |panic_info| {
            let err = panic_info
                .payload()
                .downcast_ref::<&str>()
                .map(|s| String::from(*s))
                .or(panic_info
                    .payload()
                    .downcast_ref::<String>()
                    .map(|s| s.to_owned()));

            mq.send(Msg::PWEvent(PWEvent::ThreadPanic(err))).unwrap();
        }
    }));

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

    print_waybar_status(false);

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
        let ret = epoll.wait(&mut events, EpollTimeout::NONE);

        epoll.delete(wayland_read_guard.connection_fd())?;

        let event = match ret {
            Ok(_) => events[0],
            Err(Errno::EINTR) => continue,
            Err(err) => Err(err)?,
        };

        match event.data().into() {
            MessageQueueType::Main => {
                std::mem::drop(wayland_read_guard);
                mq_receiver.recv()?.handle(
                    pw_thread,
                    &mut inhibit_idle_state_manager,
                    &mut wayland_idle_inhibitor,
                )?;
            }

            MessageQueueType::Wayland => {
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
            MessageQueueType::Main => mq_receiver.recv()?.handle(
                pw_thread,
                &mut inhibit_idle_state_manager,
                idle_inhibitor.as_mut(),
            )?,

            MessageQueueType::Unknown => log::error!(target: "main", "Unknown event queue"),

            MessageQueueType::Wayland => unreachable!(),
        }
    }
    Ok(())
}
