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

use regex::Regex;

use super::graph::NodeData;

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
        return true;
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
        return false;
    }
}

pub struct SinkFilter {
    name: Option<Regex>,
}

impl SinkFilter {
    pub fn new(name: Option<Regex>) -> Self {
        Self { name }
    }
}

impl Filter<NodeData> for SinkFilter {
    fn matches(&self, node: &NodeData) -> bool {
        if let Some(name) = &self.name {
            if !name.is_match(&node.get_name()) {
                return false;
            }
        }

        return true;
    }
}

pub struct NodeFilter {
    name: Option<Regex>,
    media_class: Option<Regex>,
    media_role: Option<Regex>,
    media_software: Option<Regex>,
}

impl NodeFilter {
    pub fn new(
        name: Option<Regex>,
        media_class: Option<Regex>,
        media_role: Option<Regex>,
        media_software: Option<Regex>,
    ) -> Self {
        Self {
            name,
            media_class,
            media_role,
            media_software,
        }
    }
}

impl Filter<NodeData> for NodeFilter {
    fn matches(&self, node: &NodeData) -> bool {
        if let Some(name) = &self.name {
            if !name.is_match(&node.get_name()) {
                return false;
            }
        }

        if let Some(media_class) = &self.media_class {
            if !media_class.is_match(&node.media_class.clone().unwrap_or(String::default())) {
                return false;
            }
        }

        if let Some(media_role) = &self.media_role {
            if !media_role.is_match(&node.media_role.clone().unwrap_or(String::default())) {
                return false;
            }
        }

        if let Some(media_software) = &self.media_software {
            if !media_software.is_match(&node.media_software.clone().unwrap_or(String::default())) {
                return false;
            }
        }

        return true;
    }
}
