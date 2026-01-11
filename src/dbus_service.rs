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

use crate::message_queue::MessageQueueSender;
use crate::Msg;
use std::error::Error;
use zbus::{interface, ConnectionBuilder};

pub struct DbusService {
    mq: MessageQueueSender<Msg>,
}

impl DbusService {
    pub fn new(mq: MessageQueueSender<Msg>) -> Self {
        Self { mq }
    }
}

#[interface(name = "org.wayland.IdleInhibit.Control")]
impl DbusService {
    async fn toggle_manual(&self) {
        log::debug!("D-Bus method 'ToggleManual' called.");
        if let Err(e) = self.mq.send(Msg::ToggleManual) {
            log::error!("Failed to send ToggleManual message: {}", e);
        }
    }
}

pub async fn start_dbus_service(mq: MessageQueueSender<Msg>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let dbus_service = DbusService::new(mq);
    let _connection = ConnectionBuilder::session()?
        .name("org.wayland.IdleInhibit.Control")?
        .serve_at("/org/wayland/IdleInhibit/Control", dbus_service)?
        .build()
        .await?;

    log::info!("D-Bus service for manual toggle started successfully.");
    std::future::pending::<()>().await;

    Ok(())
}
