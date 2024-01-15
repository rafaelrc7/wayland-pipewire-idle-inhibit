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

use pipewire::{
    link::{Link, LinkListener},
    node::{Node, NodeListener},
    port::{Port, PortListener},
    proxy::{Listener, ProxyT},
    spa::Direction,
};

pub type Id = u32;

pub struct Proxy<TProxy: ProxyT, TListener: Listener> {
    pub proxy: TProxy,
    pub listener: TListener,
}

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
    pub fn get_name(&self) -> &str {
        if let Some(description) = &self.description {
            description
        } else if let Some(nick) = &self.nick {
            nick
        } else if let Some(name) = &self.name {
            name
        } else {
            ""
        }
    }

    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.app_name.is_none()
            && self.description.is_none()
            && self.nick.is_none()
            && self.media_class.is_none()
            && self.media_role.is_none()
            && self.media_software.is_none()
    }

    pub fn update(&mut self, new: Self) {
        if let Some(name) = new.name {
            self.name = Some(name);
        }

        if let Some(app_name) = new.app_name {
            self.app_name = Some(app_name);
        }

        if let Some(description) = new.description {
            self.description = Some(description);
        }

        if let Some(nick) = new.nick {
            self.nick = Some(nick);
        }

        if let Some(media_class) = new.media_class {
            self.media_class = Some(media_class);
        }

        if let Some(media_role) = new.media_role {
            self.media_role = Some(media_role);
        }

        if let Some(media_software) = new.media_software {
            self.media_software = Some(media_software);
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct PortData {
    pub name: Option<String>,
    pub node_id: Option<Id>,
    pub direction: Option<Direction>,
    pub is_terminal: Option<bool>,
}

impl PortData {
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.node_id.is_none()
            && self.direction.is_none()
            && self.is_terminal.is_none()
    }

    pub fn update(&mut self, new: Self) {
        if let Some(name) = new.name {
            self.name = Some(name);
        }

        if let Some(node_id) = new.node_id {
            self.node_id = Some(node_id);
        }

        if let Some(direction) = new.direction {
            self.direction = Some(direction);
        }

        if let Some(is_terminal) = new.is_terminal {
            self.is_terminal = Some(is_terminal);
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct LinkData {
    pub input_port: Option<Id>,
    pub output_port: Option<Id>,
    pub active: Option<bool>,
}

impl LinkData {
    pub fn is_empty(&self) -> bool {
        self.input_port.is_none() && self.output_port.is_none() && self.active.is_none()
    }

    pub fn update(&mut self, new: Self) {
        if let Some(input_port) = new.input_port {
            self.input_port = Some(input_port);
        }

        if let Some(output_port) = new.output_port {
            self.output_port = Some(output_port);
        }

        if let Some(active) = new.active {
            self.active = Some(active);
        }
    }
}

pub enum PWObjectData {
    Node(NodeData),
    Port(PortData),
    Link(LinkData),
}

impl PWObjectData {
    pub fn is_empty(&self) -> bool {
        match self {
            PWObjectData::Node(data) => data.is_empty(),
            PWObjectData::Port(data) => data.is_empty(),
            PWObjectData::Link(data) => data.is_empty(),
        }
    }
}

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
