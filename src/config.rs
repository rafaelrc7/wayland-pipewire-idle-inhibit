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

use std::path::PathBuf;

use chrono::Duration;
use clap::Parser;
use figment::{
    providers::{Format, Serialized, Toml},
    Figment,
};
use log::LevelFilter;
use serde::Deserialize;

use crate::args::Args;
use crate::pipewire_connection::graph_filter::{NodeFilter, SinkFilter};

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
}

impl Settings {
    pub fn new(path: Option<PathBuf>) -> Self {
        let path = match path {
            Some(path) => path,
            None => xdg::BaseDirectories::with_prefix(env!("CARGO_PKG_NAME"))
                .unwrap()
                .place_config_file("config.toml")
                .expect("Canno crate configuration file at default directory"),
        };

        let settings = Figment::new()
            .merge(Toml::file(path))
            .merge(Serialized::defaults(Args::parse()))
            .extract();

        match settings {
            Ok(settings) => settings,
            Err(error) => panic!("{}", error),
        }
    }

    pub fn get_media_minimum_duration(&self) -> Option<Duration> {
        match self.media_minimum_duration {
            0 => None,
            d => Some(Duration::seconds(d)),
        }
    }

    pub fn get_verbosity(&self) -> LevelFilter {
        self.verbosity
    }

    pub fn get_sink_whitelist(&self) -> &Vec<SinkFilter> {
        &self.sink_whitelist
    }

    pub fn get_node_blacklist(&self) -> &Vec<NodeFilter> {
        &self.node_blacklist
    }
}

fn defalt_media_minimum_duration() -> i64 {
    5
}

fn default_verbosity() -> LevelFilter {
    LevelFilter::Warn
}
