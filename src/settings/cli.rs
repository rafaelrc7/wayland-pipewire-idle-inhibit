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

//! CLI Args parsing and processing

use std::fmt::Display;

use clap::{builder::PossibleValue, Parser, ValueEnum};
use log::LevelFilter;
use serde::{Deserialize, Serialize};

use super::IdleInhibitor;

/// Struct used to derive, parse and serialise CLI args. Some of the fields will not be used by the
/// application and are only relevant in the context of CLI arguments, and thus have their
/// serialisation skipped.
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
        help="Log verbosity"
    )]
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

    #[arg(
        short = 'i',
        long = "idle-inhibitor",
        value_name = "IDLE INHIBITOR BACKEND",
        default_value_if("dbus", true.to_string(), IdleInhibitor::DBus.to_string()),
        default_value_if("wayland", true.to_string(), IdleInhibitor::Wayland.to_string()),
        default_value_if("dry_run", true.to_string(), IdleInhibitor::DryRun.to_string()),
        help = format!("Sets what idle inhibitor backend to use [default: {}]", super::default_idle_inhibitor())
    )]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    idle_inhibitor: Option<IdleInhibitor>,

    #[arg(
        short = 'b',
        long = "d-bus",
        conflicts_with = "wayland",
        conflicts_with = "dry_run",
        conflicts_with = "idle_inhibitor",
        help = "Enable DBus (org.freedesktop.ScreenSaver) idle inhibitor"
    )]
    #[serde(skip_serializing)]
    #[serde(default)]
    dbus: bool,

    #[arg(
        short = 'w',
        long = "wayland",
        conflicts_with = "dbus",
        conflicts_with = "dry_run",
        conflicts_with = "idle_inhibitor",
        help = "Enable Wayland idle inhibitor"
    )]
    #[serde(skip_serializing)]
    #[serde(default)]
    wayland: bool,

    #[arg(
        short = 'n',
        long = "dry-run",
        conflicts_with = "dbus",
        conflicts_with = "wayland",
        conflicts_with = "idle_inhibitor",
        help = "Only logs (at INFO level) about idle inhibitor state changes"
    )]
    #[serde(skip_serializing)]
    #[serde(default)]
    dry_run: bool,

    #[arg(short, long, value_name = "PATH", help = "Path to config file")]
    #[serde(skip_serializing)]
    pub config: Option<String>,
}

/// Wrapper type around [LevelFilter] to implement the trait [ValueEnum] for better CLI args
/// integration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LogLevel(LevelFilter);

impl Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl ValueEnum for LogLevel {
    fn value_variants<'a>() -> &'a [Self] {
        // TODO: Use macros to generate this array
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
