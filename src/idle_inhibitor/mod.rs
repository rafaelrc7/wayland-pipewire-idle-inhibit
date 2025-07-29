// Copyright (C) 2024-2025  Rafael Carvalho <contact@rafaelrc.com>

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

use std::error::Error;

pub mod dbus;
pub mod dry;
pub mod wayland;

pub trait IdleInhibitor {
    /// Inhibit Idle, does nothing if idle is already inhibited
    fn inhibit(&mut self) -> Result<(), Box<dyn Error>>;

    /// Uninhibit Idle, does nothing if idle is not inhibited
    fn uninhibit(&mut self) -> Result<(), Box<dyn Error>>;
}
