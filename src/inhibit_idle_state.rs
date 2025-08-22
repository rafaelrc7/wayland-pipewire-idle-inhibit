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

//! Helper to manage the idle inhibiting state. This module is used to treat PipeWire events and
//! send messages if and when idle should be inhibited, treating the minimum sound duration.

use chrono::Duration;
use log::{debug, trace};
use timer::{Guard, Timer};

use crate::message_queue::MessageQueueSender;

/// Module Event message type
#[derive(Clone, Copy, Debug)]
pub enum InhibitIdleStateEvent {
    InhibitIdle(bool),
    AudioInhibitTimerFired,
}

/// Manager of the idle inhibit state
pub struct InhibitIdleState<Msg: From<InhibitIdleStateEvent> + Clone> {
    inhibit_idle_timout_callback: Timer,
    inhibit_idle_timout_callback_guard: Option<Guard>,
    inhibit_idle_timout: Option<Duration>,
    is_audio_inhibited: bool,
    is_manual_inhibited: bool,
    is_inhibited: bool,
    inhibit_idle_callback: MessageQueueSender<Msg>,
}

impl<Msg: From<InhibitIdleStateEvent> + Clone + Send + 'static> InhibitIdleState<Msg> {
    pub fn new(
        inhibit_idle_timout: Option<Duration>,
        inhibit_idle_callback: MessageQueueSender<Msg>,
    ) -> Self {
        Self {
            inhibit_idle_timout_callback: Timer::new(),
            inhibit_idle_timout_callback_guard: None,
            inhibit_idle_timout,
            is_audio_inhibited: false,
            is_manual_inhibited: false,
            is_inhibited: false,
            inhibit_idle_callback,
        }
    }

    pub fn toggle_manual_inhibit(&mut self) {

        self.is_manual_inhibited = !self.is_manual_inhibited;
        debug!(target: "InhibitIdleState", "Manual inhibit toggled to: {}", self.is_manual_inhibited);
        self.update_is_idle_inhibited();
    }

    pub fn set_is_audio_inhibited(&mut self, is_audio_inhibited: bool) {
        if let (Some(inhibit_idle_timout), true) = (self.inhibit_idle_timout, is_audio_inhibited) {
            if self.inhibit_idle_timout_callback_guard.is_some() {
                trace!(target: "InhibitIdleState::set_is_audio_inhibited", "Update Timer is already running");
                return;
            }

            debug!(target: "InhibitIdleState::set_is_audio_inhibited", "Started Timer to inhibit idling");
            let callback = self.inhibit_idle_callback.clone();
            self.inhibit_idle_timout_callback_guard = Some(
                self.inhibit_idle_timout_callback
                    .schedule_with_delay(inhibit_idle_timout, move || {
                        callback.send(InhibitIdleStateEvent::AudioInhibitTimerFired.into()).unwrap();
                    }),
            );
        } else {
            if self.inhibit_idle_timout_callback_guard.is_some() {
                self.inhibit_idle_timout_callback_guard = None
            }
            self.is_audio_inhibited = is_audio_inhibited;
            self.update_is_idle_inhibited();
        }
    }

    pub fn set_is_inhibited_from_timer(&mut self) {
        self.is_audio_inhibited = true;
        self.update_is_idle_inhibited();
    }

    fn update_is_idle_inhibited(&mut self) {
        let should_inhibit = self.is_audio_inhibited || self.is_manual_inhibited;

        if self.is_inhibited == should_inhibit {
            trace!(target: "InhibitIdleState", "Tried to update 'is_idle_inhibited', but value is the same");
            return;
        }

        self.is_inhibited = should_inhibit;
        self.inhibit_idle_callback
            .send(Msg::from(InhibitIdleStateEvent::InhibitIdle(
                should_inhibit,
            )))
            .unwrap();
    }
}
