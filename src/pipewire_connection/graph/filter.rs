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

//! Tools used for filtering over [super::PWGraph] objects.

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::NodeData;

/// Represents a generic filter for a generic type. In the contexts of this application, it is used
/// to filter objects of the [super::PWGraph], mainly [super::NodeData]s.
pub trait Filter<T> {
    /// Checks if the filter matches a given object.
    fn matches(&self, data: &T) -> bool;

    /// Checks if all filters of a slice matches a object.
    ///
    /// This function will return false on the first failed filter and true if all checks succed.
    ///
    /// When an empty slice of filters is passed, it returns true.
    fn matches_all(filters: &[Self], data: &T) -> bool
    where
        Self: Sized,
    {
        filters.iter().all(|f| f.matches(data))
    }

    /// Checks if any filters of a slice matches a object.
    ///
    /// This function will return true on the first succesful filter and false if all checks fail.
    ///
    /// When an empty slice of filters is passed, it returns false.
    fn matches_any(filters: &[Self], data: &T) -> bool
    where
        Self: Sized,
    {
        filters.iter().any(|f| f.matches(data))
    }
}

/// Checks if a [Regex] filter matches a given [String] property.
///
/// If the filter is [None] this means it should not be applied, and thus the result is always
/// true.
///
/// If the filter is [Some] but the property is [None], it means the filter must be applied but the
/// property is missing, thus the result is always false.
///
/// If the filter and property are [Some], the result will be the answer to if the property value
/// matches the filter [Regex].
fn matches_property(filter: &Option<Regex>, property: Option<&str>) -> bool {
    filter
        .as_ref()
        .map_or(true, |f| property.map_or(false, |p| f.is_match(p)))
}

/// Represents a [Filter] over a Sink. A Sink is a special case of a Node, and thus filters over
/// [super::NodeData]s.
#[derive(Serialize, Deserialize, Clone)]
pub struct SinkFilter {
    #[serde(default, with = "serde_regex")]
    name: Option<Regex>,
}

impl Filter<NodeData> for SinkFilter {
    fn matches(&self, node: &NodeData) -> bool {
        matches_property(&self.name, node.get_name())
    }
}

/// Represents a [Filter] over a generic Node, and thus filters over [super::NodeData]s.
#[derive(Serialize, Deserialize, Clone)]
pub struct NodeFilter {
    #[serde(default, with = "serde_regex")]
    name: Option<Regex>,

    #[serde(default, with = "serde_regex")]
    app_name: Option<Regex>,

    #[serde(default, with = "serde_regex")]
    media_class: Option<Regex>,

    #[serde(default, with = "serde_regex")]
    media_role: Option<Regex>,

    #[serde(default, with = "serde_regex")]
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
