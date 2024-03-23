// Copyright (C) 2023-2024  Rafael Carvalho <contact@rafaelrc.com>

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

//! Module responsible with the tool's configuration

use std::{error::Error, path::PathBuf};

use chrono::Duration;
use clap::Parser;
use figment::{
    providers::{Format, Serialized, Toml},
    Figment,
};
use log::LevelFilter;
use serde::Deserialize;

use crate::pipewire_connection::graph::filter::{NodeFilter, SinkFilter};

mod cli;
use cli::Args;

/// Struct that stores the settings that affect the tool behaviour
#[derive(Deserialize)]
pub struct Settings {
    #[serde(default = "defalt_media_minimum_duration")]
    media_minimum_duration: i64,

    #[serde(default = "default_verbosity")]
    verbosity: LevelFilter,

    #[serde(default)]
    sink_whitelist: Vec<SinkFilter>,

    #[serde(default)]
    node_blacklist: Vec<NodeFilter>,

    #[serde(default = "default_wayland")]
    wayland: bool,
}

impl Settings {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let cli = Args::parse();

        let config_path = match cli.config {
            Some(ref p) => PathBuf::from(p),
            None => xdg::BaseDirectories::with_prefix(env!("CARGO_PKG_NAME"))?
                .place_config_file("config.toml")?,
        };

        let settings = Figment::new()
            .merge(Toml::file(config_path))
            .merge(Serialized::defaults(cli))
            .extract()?;

        Ok(settings)
    }

    /// Getter for the media minimum duration with the [chrono::Duration] type. If the set duration
    /// is 0, [None] is returned, to easily detect if this check is necessary
    pub fn get_media_minimum_duration(&self) -> Option<Duration> {
        match self.media_minimum_duration {
            0 => None,
            d => Some(Duration::seconds(d)),
        }
    }

    /// Returns the current log verbosity
    pub fn get_verbosity(&self) -> LevelFilter {
        self.verbosity
    }

    /// Return sink filters
    pub fn get_sink_whitelist(&self) -> &Vec<SinkFilter> {
        &self.sink_whitelist
    }

    /// Return Node filters
    pub fn get_node_blacklist(&self) -> &Vec<NodeFilter> {
        &self.node_blacklist
    }

    pub fn is_wayland_enabled(&self) -> bool {
        self.wayland
    }
}

/// Default media minimum duration, set to 5 seconds
fn defalt_media_minimum_duration() -> i64 {
    5
}

/// Default log verbosity, set to [LevelFilter::Warn]
fn default_verbosity() -> LevelFilter {
    LevelFilter::Warn
}

fn default_wayland() -> bool {
    true
}
