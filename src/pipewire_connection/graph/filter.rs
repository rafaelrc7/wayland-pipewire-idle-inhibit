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

    fn matches_all(filters: &[Self], data: &T) -> bool
    where
        Self: Sized,
    {
        filters.iter().all(|f| f.matches(data))
    }

    fn matches_any(filters: &[Self], data: &T) -> bool
    where
        Self: Sized,
    {
        filters.iter().any(|f| f.matches(data))
    }
}

fn matches_property(filter: &Option<Regex>, property: Option<&str>) -> bool {
    filter
        .as_ref()
        .map_or(true, |f| property.map_or(false, |p| f.is_match(p)))
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SinkFilter {
    #[serde(with = "serde_regex")]
    #[serde(default)]
    name: Option<Regex>,
}

impl Filter<NodeData> for SinkFilter {
    fn matches(&self, node: &NodeData) -> bool {
        matches_property(&self.name, node.get_name())
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
        matches_property(&self.name, node.get_name())
            && matches_property(&self.app_name, node.app_name.as_deref())
            && matches_property(&self.media_class, node.media_class.as_deref())
            && matches_property(&self.media_role, node.media_role.as_deref())
            && matches_property(&self.media_software, node.media_software.as_deref())
    }
}
