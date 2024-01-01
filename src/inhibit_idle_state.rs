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

use std::sync::{mpsc, Arc, RwLock};

use chrono::Duration;
use log::{debug, trace};
use timer::{Guard, Timer};

#[derive(Debug)]
pub enum InhibitIdleStateEvent {
    InhibitIdle(bool),
}

pub struct InhibitIdleState<Msg: From<InhibitIdleStateEvent>> {
    inhibit_idle_timout_callback: Timer,
    inhibit_idle_timout_callback_guard: Option<Guard>,
    inhibit_idle_timout: Option<Duration>,
    is_idle_inhibited: Arc<RwLock<bool>>,
    inhibit_idle_callback: mpsc::Sender<Msg>,
}

impl<Msg: From<InhibitIdleStateEvent> + Send + 'static> InhibitIdleState<Msg> {
    pub fn new(
        inhibit_idle_timout: Option<Duration>,
        inhibit_idle_callback: mpsc::Sender<Msg>,
    ) -> Self {
        Self {
            inhibit_idle_timout_callback: Timer::new(),
            inhibit_idle_timout_callback_guard: None,
            inhibit_idle_timout,
            is_idle_inhibited: Arc::new(RwLock::new(false)),
            inhibit_idle_callback,
        }
    }

    pub fn set_is_idle_inhibited(&mut self, is_idle_inhibited: bool) {
        if let (Some(inhibit_idle_timout), true) = (self.inhibit_idle_timout, is_idle_inhibited) {
            if self.inhibit_idle_timout_callback_guard.is_some() {
                trace!(target: "InhibitIdleState::set_is_idle_inhibited", "Update Timer is already running");
                return;
            }

            debug!(target: "InhibitIdleState::set_is_idle_inhibited", "Started Timer to inhibit idling");
            self.inhibit_idle_timout_callback_guard = Some(
                self.inhibit_idle_timout_callback
                    .schedule_with_delay(inhibit_idle_timout, {
                        let is_idle_inhibited_ref = Arc::clone(&self.is_idle_inhibited);
                        let inhibit_idle_callback = self.inhibit_idle_callback.clone();
                        move || {
                            let is_idle_inhibited_ref = &is_idle_inhibited_ref;
                            Self::update_is_idle_inhibited(
                                Arc::clone(is_idle_inhibited_ref),
                                inhibit_idle_callback.clone(),
                                is_idle_inhibited,
                            );
                        }
                    }),
            );
        } else {
            if self.inhibit_idle_timout_callback_guard.is_some() {
                self.inhibit_idle_timout_callback_guard = None
            }
            Self::update_is_idle_inhibited(
                self.is_idle_inhibited.clone(),
                self.inhibit_idle_callback.clone(),
                is_idle_inhibited,
            );
        }
    }

    fn update_is_idle_inhibited(
        is_idle_inhibited_ref: Arc<RwLock<bool>>,
        inhibit_idle_callback: mpsc::Sender<Msg>,
        is_idle_inhibited: bool,
    ) {
        if *is_idle_inhibited_ref.read().unwrap() == is_idle_inhibited {
            trace!(target: "InhibitIdleState", "Tried to update 'is_idle_inhibited', but value is the same");
            return;
        }

        *is_idle_inhibited_ref.write().unwrap() = is_idle_inhibited;
        inhibit_idle_callback
            .send(Msg::from(InhibitIdleStateEvent::InhibitIdle(
                is_idle_inhibited,
            )))
            .unwrap();
        debug!(target: "InhibitIdleState", "Idle inhibting was {}", if is_idle_inhibited { "ENABLED" } else { "DISABLED" });
    }
}
