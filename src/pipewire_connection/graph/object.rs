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
    pub fn get_name(&self) -> String {
        if let Some(description) = &self.description {
            description.clone()
        } else if let Some(nick) = &self.nick {
            nick.clone()
        } else if let Some(name) = &self.name {
            name.clone()
        } else {
            String::default()
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

    pub fn join(old: Self, new: Self) -> Self {
        let NodeData {
            name: old_name,
            app_name: old_app_name,
            description: old_description,
            nick: old_nick,
            media_class: old_media_class,
            media_role: old_media_role,
            media_software: old_media_software,
        } = old;
        let NodeData {
            name: new_name,
            app_name: new_app_name,
            description: new_description,
            nick: new_nick,
            media_class: new_media_class,
            media_role: new_media_role,
            media_software: new_media_software,
        } = new;

        NodeData {
            name: match new_name {
                None => old_name,
                new_name => new_name,
            },
            app_name: match new_app_name {
                None => old_app_name,
                new_app_name => new_app_name,
            },
            description: match new_description {
                None => old_description,
                new_description => new_description,
            },
            nick: match new_nick {
                None => old_nick,
                new_nick => new_nick,
            },
            media_class: match new_media_class {
                None => old_media_class,
                new_media_class => new_media_class,
            },
            media_role: match new_media_role {
                None => old_media_role,
                new_media_role => new_media_role,
            },
            media_software: match new_media_software {
                None => old_media_software,
                new_media_software => new_media_software,
            },
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

    pub fn join(old: Self, new: Self) -> Self {
        let PortData {
            name: old_name,
            node_id: old_node_id,
            direction: old_direction,
            is_terminal: old_is_terminal,
        } = old;
        let PortData {
            name: new_name,
            node_id: new_node_id,
            direction: new_direction,
            is_terminal: new_is_terminal,
        } = new;

        PortData {
            name: match new_name {
                None => old_name,
                new_name => new_name,
            },
            node_id: match new_node_id {
                None => old_node_id,
                new_node_id => new_node_id,
            },
            direction: match new_direction {
                None => old_direction,
                new_direction => new_direction,
            },
            is_terminal: match new_is_terminal {
                None => old_is_terminal,
                new_is_terminal => new_is_terminal,
            },
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

    pub fn join(old: Self, new: Self) -> Self {
        let LinkData {
            input_port: old_input_port,
            output_port: old_output_port,
            active: old_active,
        } = old;
        let LinkData {
            input_port: new_input_port,
            output_port: new_output_port,
            active: new_active,
        } = new;

        LinkData {
            input_port: match new_input_port {
                None => old_input_port,
                new_input_port => new_input_port,
            },
            output_port: match new_output_port {
                None => old_output_port,
                new_output_port => new_output_port,
            },
            active: match new_active {
                None => old_active,
                new_active => new_active,
            },
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
