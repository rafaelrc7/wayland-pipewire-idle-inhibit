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

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::NodeData;

pub trait Filter<T> {
    fn matches(&self, data: &T) -> bool;

    fn matches_all(filters: &Vec<Self>, data: &T) -> bool
    where
        Self: Sized,
    {
        for filter in filters {
            if !filter.matches(data) {
                return false;
            }
        }
        true
    }

    fn matches_any(filters: &Vec<Self>, data: &T) -> bool
    where
        Self: Sized,
    {
        for filter in filters {
            if filter.matches(data) {
                return true;
            }
        }
        false
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SinkFilter {
    #[serde(with = "serde_regex")]
    #[serde(default)]
    name: Option<Regex>,
}

impl Filter<NodeData> for SinkFilter {
    fn matches(&self, node: &NodeData) -> bool {
        if let Some(name) = &self.name {
            if !name.is_match(&node.get_name()) {
                return false;
            }
        }

        true
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NodeFilter {
    #[serde(with = "serde_regex")]
    #[serde(default)]
    name: Option<Regex>,

    #[serde(with = "serde_regex")]
    #[serde(default)]
    app_name: Option<Regex>,

    #[serde(with = "serde_regex")]
    #[serde(default)]
    media_class: Option<Regex>,

    #[serde(with = "serde_regex")]
    #[serde(default)]
    media_role: Option<Regex>,

    #[serde(with = "serde_regex")]
    #[serde(default)]
    media_software: Option<Regex>,
}

impl Filter<NodeData> for NodeFilter {
    fn matches(&self, node: &NodeData) -> bool {
        if let Some(name) = &self.name {
            if !name.is_match(&node.get_name()) {
                return false;
            }
        }

        if let Some(app_name) = &self.app_name {
            if !app_name.is_match(&node.app_name.clone().unwrap_or_default()) {
                return false;
            }
        }

        if let Some(media_class) = &self.media_class {
            if !media_class.is_match(&node.media_class.clone().unwrap_or_default()) {
                return false;
            }
        }

        if let Some(media_role) = &self.media_role {
            if !media_role.is_match(&node.media_role.clone().unwrap_or_default()) {
                return false;
            }
        }

        if let Some(media_software) = &self.media_software {
            if !media_software.is_match(&node.media_software.clone().unwrap_or_default()) {
                return false;
            }
        }

        true
    }
}
