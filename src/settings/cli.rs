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

use std::fmt::Display;

use clap::{builder::PossibleValue, Parser, ValueEnum};
use log::LevelFilter;
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug, Serialize, Deserialize)]
#[command(author, version, about)]
pub struct Args {
    #[arg(
        short = 'd',
        long,
        value_name = "SECONDS",
        allow_negative_numbers = false,
        help = "Minimum media duration to inhibit idle"
    )]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    media_minimum_duration: Option<i64>,

    #[arg(
        short,
        long,
        default_value_if("quiet", true.to_string(), LogLevel(LevelFilter::Off).to_string()),
        help="Log verbosity")]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    verbosity: Option<LogLevel>,

    #[arg(
        short,
        long,
        conflicts_with = "verbosity",
        help = "Disables logging completely"
    )]
    #[serde(skip_serializing)]
    #[serde(default)]
    quiet: bool,

    #[arg(short, long, value_name = "PATH", help = "Path to config file")]
    #[serde(skip_serializing)]
    pub config: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LogLevel(LevelFilter);

impl Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl ValueEnum for LogLevel {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self(LevelFilter::Off),
            Self(LevelFilter::Error),
            Self(LevelFilter::Warn),
            Self(LevelFilter::Info),
            Self(LevelFilter::Debug),
            Self(LevelFilter::Trace),
        ]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(PossibleValue::new(self.0.to_string()))
    }
}
