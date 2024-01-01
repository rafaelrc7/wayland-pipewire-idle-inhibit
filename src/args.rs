// Copyright (C) 2024  Rafael Carvalho <contact@rafaelrc.com>

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

use std::fmt::Display;

use chrono::Duration;
use clap::{builder::PossibleValue, Parser, ValueEnum};
use log::LevelFilter;

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Args {
    #[arg(
        short = 'd',
        long,
        value_name = "SECONDS",
        default_value_t = 5,
        allow_negative_numbers = false,
        help = "Minimum media duration to inhibit idle"
    )]
    media_minimum_duration: i64,

    #[arg(
        short,
        long,
        default_value_t = LogLevel(LevelFilter::Warn),
        default_value_if("quiet", "true", "quiet"),
        help="Log verbosity")]
    verbosity: LogLevel,

    #[arg(
        short,
        long,
        conflicts_with = "verbosity",
        help = "Disables logging completely"
    )]
    quiet: bool,
}

impl Args {
    pub fn get_log_level(&self) -> LevelFilter {
        let LogLevel(level_filter) = self.verbosity;
        level_filter
    }

    pub fn get_media_minimun_duration(&self) -> Option<Duration> {
        match self.media_minimum_duration {
            0 => None,
            d => Some(Duration::seconds(d)),
        }
    }
}

#[derive(Debug, Clone)]
struct LogLevel(LevelFilter);

impl Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(level_filter) = self;
        match level_filter {
            LevelFilter::Off => write!(f, "off"),
            LevelFilter::Error => write!(f, "error"),
            LevelFilter::Warn => write!(f, "warn"),
            LevelFilter::Info => write!(f, "info"),
            LevelFilter::Debug => write!(f, "debug"),
            LevelFilter::Trace => write!(f, "trace"),
        }
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

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        let LogLevel(level_filter) = self;
        match level_filter {
            LevelFilter::Off => Some(PossibleValue::new("quiet")),
            LevelFilter::Error => Some(PossibleValue::new("error")),
            LevelFilter::Warn => Some(PossibleValue::new("warn")),
            LevelFilter::Info => Some(PossibleValue::new("info")),
            LevelFilter::Debug => Some(PossibleValue::new("debug")),
            LevelFilter::Trace => Some(PossibleValue::new("trace")),
        }
    }
}
