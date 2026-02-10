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

use crate::message_queue::MessageQueueSender;
use crate::Msg;
use zbus::interface;

pub struct DBusServer {
    mq: MessageQueueSender<Msg>,
    manual_inhibit: bool,
    effective_inhibit: bool,
}

impl DBusServer {
    pub fn new(mq: MessageQueueSender<Msg>) -> Self {
        Self {
            mq,
            manual_inhibit: false,
            effective_inhibit: false,
        }
    }

    // Update the internal effective state for the D-Bus property
    pub fn set_effective_inhibit(&mut self, value: bool) {
        self.effective_inhibit = value;
    }

    pub fn get_effective_inhibit(&self) -> bool {
        self.effective_inhibit
    }

    // Update the internal effective state for the D-Bus property
    pub fn get_manual_inhibit(&self) -> bool {
        self.manual_inhibit
    }
}

#[interface(name = "com.rafaelrc.WaylandPipewireIdleInhibit")]
impl DBusServer {
    #[zbus(property)]
    fn manual_inhibit(&self) -> bool {
        self.manual_inhibit
    }

    #[zbus(property)]
    fn set_manual_inhibit(&mut self, value: bool) {
        if self.manual_inhibit != value {
            self.manual_inhibit = value;
            // Send message to the main loop to re-evaluate inhibition state
            self.mq.send(Msg::ManualInhibit(value)).unwrap();
        }
    }

    #[zbus()]
    fn toggle_manual_inhibit(&mut self) {
        let new_val = !self.manual_inhibit;
        self.manual_inhibit = new_val;
        self.mq.send(Msg::ManualInhibit(new_val)).unwrap();
    }

    #[zbus(property)]
    fn is_idle_inhibited(&self) -> bool {
        self.effective_inhibit
    }
}
