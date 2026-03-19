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

use std::sync::{Arc, RwLock};

use chrono::Duration;
use log::{debug, trace};
use timer::{Guard, Timer};

use crate::message_queue::MessageQueueSender;

/// Module Event message type
#[derive(Clone, Copy, Debug)]
pub enum InhibitIdleStateEvent {
    InhibitIdle(bool),
    TimeoutExpired,
}

/// Manager of the idle inhibit state
pub struct InhibitIdleState<Msg: From<InhibitIdleStateEvent> + Clone> {
    inhibit_idle_timout_callback: Timer,
    inhibit_idle_timout_callback_guard: Option<Guard>,
    inhibit_idle_timout: Option<Duration>,
    pw_inhibit: bool,
    manual_inhibit: bool,
    is_idle_inhibited: Arc<RwLock<bool>>,
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
            pw_inhibit: false,
            manual_inhibit: false,
            is_idle_inhibited: Arc::new(RwLock::new(false)),
            inhibit_idle_callback,
        }
    }

    pub fn set_manual_inhibit(&mut self, value: bool) {
        self.manual_inhibit = value;

        // Force an update event because manual state changed, even if effective state (bool) might be same
        self.reevaluate_effective_state(true);
    }

    pub fn handle_timeout(&mut self) {
        if self.inhibit_idle_timout_callback_guard.is_some() {
            debug!(target: "InhibitIdleState::handle_timeout", "Timer expired, locking inhibition state");
            self.inhibit_idle_timout_callback_guard = None;
            self.pw_inhibit = true;
            self.reevaluate_effective_state(false);
        }
    }

    pub fn set_is_idle_inhibited(&mut self, is_idle_inhibited: bool) {
        if let (Some(inhibit_idle_timout), true) = (self.inhibit_idle_timout, is_idle_inhibited) {
            if self.pw_inhibit {
                return;
            }

            if self.inhibit_idle_timout_callback_guard.is_some() {
                trace!(target: "InhibitIdleState::set_is_idle_inhibited", "Update Timer is already running");
                return;
            }

            debug!(target: "InhibitIdleState::set_is_idle_inhibited", "Started Timer to inhibit idling");
            self.inhibit_idle_timout_callback_guard = Some(
                self.inhibit_idle_timout_callback
                    .schedule_with_delay(inhibit_idle_timout, {
                        let inhibit_idle_callback = self.inhibit_idle_callback.clone();
                        move || {
                            inhibit_idle_callback
                                .send(Msg::from(InhibitIdleStateEvent::TimeoutExpired))
                                .unwrap();
                        }
                    }),
            );
        } else {
            if self.inhibit_idle_timout_callback_guard.is_some() {
                self.inhibit_idle_timout_callback_guard = None
            }
            self.pw_inhibit = is_idle_inhibited;
            self.reevaluate_effective_state(false);
        }
    }

    fn reevaluate_effective_state(&mut self, force_emit: bool) {
        let new_effective = self.pw_inhibit || self.manual_inhibit;
        Self::update_is_idle_inhibited(
            &self.is_idle_inhibited,
            &self.inhibit_idle_callback,
            new_effective,
            force_emit,
        );
    }

    fn update_is_idle_inhibited(
        is_idle_inhibited_ref: &Arc<RwLock<bool>>,
        inhibit_idle_callback: &MessageQueueSender<Msg>,
        is_idle_inhibited: bool,
        force_emit: bool,
    ) {
        if !force_emit && *is_idle_inhibited_ref.read().unwrap() == is_idle_inhibited {
            return;
        }

        *is_idle_inhibited_ref.write().unwrap() = is_idle_inhibited;
        inhibit_idle_callback
            .send(Msg::from(InhibitIdleStateEvent::InhibitIdle(
                is_idle_inhibited,
            )))
            .unwrap();
        debug!(target: "InhibitIdleState", "Idle inhibiting was {}, force={}", if is_idle_inhibited { "ENABLED" } else { "DISABLED" }, force_emit);
    }
}
