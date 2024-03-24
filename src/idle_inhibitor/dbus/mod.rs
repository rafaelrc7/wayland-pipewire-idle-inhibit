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

use log::{debug, error, info};
use zbus::{blocking::Connection, proxy};

use super::IdleInhibitor;

#[proxy(
    default_service = "org.freedesktop.ScreenSaver",
    interface = "org.freedesktop.ScreenSaver",
    default_path = "/ScreenSaver"
)]
trait ScreenSaver {
    fn Inhibit(&self, application_name: &str, reason_for_inhibit: &str) -> zbus::Result<u32>;

    #[dbus_proxy(no_reply_expected)]
    fn UnInhibit(&self, cookie: u32) -> zbus::Result<()>;
}

pub struct DbusIdleInhibitor<'a> {
    _dbus_connection: Connection,
    dbus_proxy: ScreenSaverProxyBlocking<'a>,
    cookie: Option<u32>,
}

impl<'a> DbusIdleInhibitor<'a> {
    pub fn new() -> Result<DbusIdleInhibitor<'a>, Box<dyn Error>> {
        let dbus_connection = Connection::session()?;
        let dbus_proxy = ScreenSaverProxyBlocking::new(&dbus_connection)?;

        let mut dbus_idle_inhibitor = DbusIdleInhibitor {
            _dbus_connection: dbus_connection,
            dbus_proxy,
            cookie: None,
        };

        dbus_idle_inhibitor.inhibit()?;
        dbus_idle_inhibitor.uninhibit()?;

        debug!(target: "DbusIdleInhibitor::new", "DBus Idle Inhibitor created");
        Ok(dbus_idle_inhibitor)
    }
}

impl Drop for DbusIdleInhibitor<'_> {
    fn drop(&mut self) {
        if let Some(cookie) = self.cookie {
            if let Err(error) = self.dbus_proxy.UnInhibit(cookie) {
                error!(target: "DbusIdleInhibitor::drop", "{error}");
            }
            self.cookie = None;
        }
    }
}

impl IdleInhibitor for DbusIdleInhibitor<'_> {
    fn inhibit(&mut self) -> Result<(), Box<dyn Error>> {
        if self.cookie.is_none() {
            self.cookie = Some(
                self.dbus_proxy
                    .Inhibit(env!("CARGO_PKG_NAME"), "Media is being played")?,
            );
            info!(target: "DbusIdleInhibitor::inhibit", "Idle Inhibitor was ENABLED");
        }

        Ok(())
    }

    fn uninhibit(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(cookie) = self.cookie {
            self.dbus_proxy.UnInhibit(cookie)?;
            self.cookie = None;
            info!(target: "DbusIdleInhibitor::uninhibit", "Idle Inhibitor was DISABLED");
        }

        Ok(())
    }
}
