// Copyright (C) 2024  Rafael Carvalho <contact@rafaelrc.com>

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as published by
// the Free Software Foundation.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-only

use std::error::Error;

use log::info;

use super::IdleInhibitor;

#[derive(Default)]
pub struct DryRunIdleInhibitor {
    is_idle_inhibited: bool,
}

impl IdleInhibitor for DryRunIdleInhibitor {
    fn inhibit(&mut self) -> Result<(), Box<dyn Error>> {
        if !self.is_idle_inhibited {
            self.is_idle_inhibited = true;
            info!(target: "DryRunIdleInhibitor::inhibit", "Idle Inhibitor was ENABLED");
        }

        Ok(())
    }

    fn uninhibit(&mut self) -> Result<(), Box<dyn Error>> {
        if self.is_idle_inhibited {
            self.is_idle_inhibited = false;
            info!(target: "DryRunIdleInhibitor::inhibit", "Idle Inhibitor was DISABLED");
        }

        Ok(())
    }
}
