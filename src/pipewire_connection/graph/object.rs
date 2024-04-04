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

//! Object types used to represent [super::PWGraph] elements.

use pipewire::{
    link::{Link, LinkListener},
    node::{Node, NodeListener},
    port::{Port, PortListener},
    proxy::{Listener, ProxyT},
    spa::utils::Direction,
};

/// Type used by the [pipewire] crate API to represent object ids.
pub type Id = u32;

/// Generic struct that joins a [pipewire] [ProxyT], a reference to a global object, and its
/// respective [Listener].
pub struct Proxy<TProxy: ProxyT, TListener: Listener> {
    pub proxy: TProxy,
    pub listener: TListener,
}

/// Struct representing relevant data of a [pipewire::node::Node] used by the app.
///
/// When the global object is first registered, it comes without data, and its fields may be
/// optionally filled by update events. Thus, all fields are [Option]s.
#[derive(PartialEq, Debug, Clone)]
pub struct NodeData {
    pub name: Option<String>,
    pub app_name: Option<String>,
    pub description: Option<String>,
    pub nick: Option<String>,
    pub media_class: Option<String>,
    pub media_role: Option<String>,
    pub media_software: Option<String>,
}

impl NodeData {
    /// Gets a "pretty" node name.
    ///
    /// Based on the way [Helvum](https://gitlab.freedesktop.org/pipewire/helvum) does it. The
    /// compatibility with Helvum makes it easy to use it to make [super::filter::NodeFilter]s
    /// using the node name.
    pub fn get_name(&self) -> Option<&str> {
        self.description
            .as_deref()
            .or(self.nick.as_deref())
            .or(self.name.as_deref())
    }

    /// Updates fields if new data is give.
    ///
    /// Returns true if any field was updated, false otherwise.
    ///
    /// [pipewire] update events don't provide already existing data, only new one. Thus, only
    /// [Some] values should be used, as it represents data that should replace the current one.
    pub fn update(&mut self, new: Self) -> bool {
        let mut was_updated = false;

        if new.name.is_some() && self.name != new.name {
            self.name = new.name;
            was_updated = true;
        }

        if new.app_name.is_some() && self.app_name != new.app_name {
            self.app_name = new.app_name;
            was_updated = true;
        }

        if new.description.is_some() && self.description != new.description {
            self.description = new.description;
            was_updated = true;
        }

        if new.nick.is_some() && self.nick != new.nick {
            self.nick = new.nick;
            was_updated = true;
        }

        if new.media_class.is_some() && self.media_class != new.media_class {
            self.media_class = new.media_class;
            was_updated = true;
        }

        if new.media_role.is_some() && self.media_role != new.media_role {
            self.media_role = new.media_role;
            was_updated = true;
        }

        if new.media_software.is_some() && self.media_software != new.media_software {
            self.media_software = new.media_software;
            was_updated = true;
        }

        was_updated
    }
}

/// Struct representing relevant data of a [pipewire::port::Port] used by the app.
///
/// When the global object is first registered, it comes without data, and its fields may be
/// optionally filled by update events. Thus, all fields are [Option]s.
#[derive(PartialEq, Debug, Clone)]
pub struct PortData {
    pub name: Option<String>,
    pub node_id: Option<Id>,
    pub direction: Option<Direction>,
    pub is_terminal: Option<bool>,
}

impl PortData {
    /// Updates fields if new data is give.
    ///
    /// Returns true if any field was updated, false otherwise.
    ///
    /// [pipewire] update events don't provide already existing data, only new one. Thus, only
    /// [Some] values should be used, as it represents data that should replace the current one.
    pub fn update(&mut self, new: Self) -> bool {
        let mut was_updated = false;

        if new.name.is_some() && self.name != new.name {
            self.name = new.name;
            was_updated = true;
        }

        if new.node_id.is_some() && self.node_id != new.node_id {
            self.node_id = new.node_id;
            was_updated = true;
        }

        if new.direction.is_some() && self.direction != new.direction {
            self.direction = new.direction;
            was_updated = true;
        }

        if new.is_terminal.is_some() && self.is_terminal != new.is_terminal {
            self.is_terminal = new.is_terminal;
            was_updated = true;
        }

        was_updated
    }
}

/// Struct representing relevant data of a [pipewire::link::Link] used by the app.
///
/// When the global object is first registered, it comes without data, and its fields may be
/// optionally filled by update events. Thus, all fields are [Option]s.
#[derive(PartialEq, Debug, Clone)]
pub struct LinkData {
    pub input_port: Option<Id>,
    pub output_port: Option<Id>,
    pub active: Option<bool>,
}

impl LinkData {
    /// Updates fields if new data is give.
    ///
    /// Returns true if any field was updated, false otherwise.
    ///
    /// [pipewire] update events don't provide already existing data, only new one. Thus, only
    /// [Some] values should be used, as it represents data that should replace the current one.
    pub fn update(&mut self, new: Self) -> bool {
        let mut was_updated = false;

        if new.input_port.is_some() && self.input_port != new.input_port {
            self.input_port = new.input_port;
            was_updated = true;
        }

        if new.output_port.is_some() && self.output_port != new.output_port {
            self.output_port = new.output_port;
            was_updated = true;
        }

        if new.active.is_some() && self.active != new.active {
            self.active = new.active;
            was_updated = true;
        }

        was_updated
    }
}

/// Enum of all [PWObject] data variants. Used by polymorphic functions over only the object data.
pub enum PWObjectData {
    Node(NodeData),
    Port(PortData),
    Link(LinkData),
}

/// Enum of all tracked types of [pipewire] graph elements.
///
/// The variants are structs of the object data and its [Proxy].
pub enum PWObject {
    Node {
        data: NodeData,
        proxy: Proxy<Node, NodeListener>,
    },
    Port {
        data: PortData,
        proxy: Proxy<Port, PortListener>,
    },
    Link {
        data: LinkData,
        proxy: Proxy<Link, LinkListener>,
    },
}
