// Copyright (C) 2023-2024  Rafael Carvalho <contact@rafaelrc.com>

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

use std::collections::{HashMap, HashSet};

use pipewire::{
    link::{Link, LinkListener},
    node::{Node, NodeListener},
    port::{Port, PortListener},
    proxy::{Listener, ProxyT},
    spa::Direction,
};

use log::{debug, trace, warn};

use super::graph_filter::{Filter, NodeFilter, SinkFilter};

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
        if let Some(app_name) = &self.app_name {
            app_name.clone()
        } else if let Some(description) = &self.description {
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

impl PWObject {}

pub struct PWGraph {
    objects: HashMap<Id, PWObject>,
    sinks: HashSet<Id>,
    links_to_port: HashMap<Id, HashSet<Id>>,
    links_from_port: HashMap<Id, HashSet<Id>>,
    node_input_ports: HashMap<Id, HashSet<Id>>,
    node_output_ports: HashMap<Id, HashSet<Id>>,
    sink_whitelist: Vec<SinkFilter>,
    node_blacklist: Vec<NodeFilter>,
}

impl PWGraph {
    pub fn new(sink_whitelist: Vec<SinkFilter>, node_blacklist: Vec<NodeFilter>) -> Self {
        Self {
            objects: HashMap::default(),
            sinks: HashSet::default(),
            links_to_port: HashMap::default(),
            links_from_port: HashMap::default(),
            node_input_ports: HashMap::default(),
            node_output_ports: HashMap::default(),
            sink_whitelist,
            node_blacklist,
        }
    }

    pub fn insert(&mut self, id: Id, obj: PWObject) {
        match obj {
            PWObject::Node { ref data, .. } => {
                let NodeData {
                    ref media_class, ..
                } = data;
                debug!(target: "PWGraph::insert", "Node ({id}) '{}'; {:?}", data.get_name(), data);
                if let Some(media_class) = media_class {
                    if media_class.contains("Sink")
                        && (self.sink_whitelist.is_empty()
                            || SinkFilter::matches_any(&self.sink_whitelist, data))
                    {
                        self.sinks.insert(id);
                    }
                };
            }
            PWObject::Port { ref data, .. } => {
                let PortData {
                    node_id, direction, ..
                } = data;
                debug!(target: "PWGraph::insert", "Port ({id})");
                if let (Some(node_id), Some(direction)) = (node_id, direction) {
                    match *direction {
                        Direction::Input => {
                            debug!(target: "PWGraph::insert", "Port ({id}) as Node {node_id} Input; {:?}", data);
                            self.get_node_input_ports(node_id).insert(id);
                        }
                        Direction::Output => {
                            debug!(target: "PWGraph::insert", "Port ({id}) as Node {node_id} Output; {:?}", data);
                            self.get_node_output_ports(node_id).insert(id);
                        }
                        _ => {}
                    };
                };
            }
            PWObject::Link { ref data, .. } => {
                let LinkData {
                    input_port,
                    output_port,
                    ..
                } = data;

                debug!(target: "PWGraph::insert", "Link ({id}); {:?}", data);

                if let Some(output_port) = output_port {
                    debug!(target: "PWGraph::insert", "Link ({id}) with output_port {output_port}");
                    self.get_links_from_port(output_port).insert(id);
                };

                if let Some(input_port) = input_port {
                    debug!(target: "PWGraph::insert", "Link ({id}) with input_port {input_port}");
                    self.get_links_to_port(input_port).insert(id);
                };
            }
        }

        self.objects.insert(id, obj);
    }

    pub fn update(&mut self, id: Id, new_data: PWObjectData) -> bool {
        trace!(target: "PWGraph::update", "Called for object with ID {id}");
        let old_obj = match self.objects.remove(&id) {
            Some(o) => o,
            None => {
                warn!(target: "PWGraph::update", "Tried to update inexistent object with ID {id}");
                return false;
            }
        };

        if new_data.is_empty() {
            trace!(target: "PWGraph::update", "Tried to update object with ID {id} but new_data is empty");
            self.objects.insert(id, old_obj);
            return false;
        }

        match new_data {
            PWObjectData::Node(new_data) => {
                let PWObject::Node {
                    data: old_data,
                    proxy,
                } = old_obj
                else {
                    warn!(target: "PWGraph::update", "Tried to update Node, but object of ID {id} is not a Node");
                    self.objects.insert(id, old_obj);
                    return false;
                };

                if new_data == old_data {
                    trace!(target: "PWGraph::update", "Tried to update Node ({id}), but it is unmodified");
                    self.objects.insert(
                        id,
                        PWObject::Node {
                            data: old_data,
                            proxy,
                        },
                    );
                    return false;
                }

                let NodeData {
                    media_class: ref new_media_class,
                    ..
                } = new_data;

                let NodeData {
                    media_class: ref old_media_class,
                    ..
                } = old_data;

                if new_media_class != old_media_class {
                    if let Some(new_media_class) = new_media_class {
                        if let Some(old_media_class) = old_media_class {
                            if old_media_class.contains("Sink") {
                                self.sinks.remove(&id);
                            }
                        }
                        if new_media_class.contains("Sink")
                            && (self.sink_whitelist.is_empty()
                                || SinkFilter::matches_any(&self.sink_whitelist, &new_data))
                        {
                            self.sinks.insert(id);
                        }
                    }
                }

                let new_data = NodeData::join(old_data.clone(), new_data);
                debug!(target: "PWGraph::update", "Updated Node ({id}) {:?} -> {:?}", old_data, new_data);
                self.objects.insert(
                    id,
                    PWObject::Node {
                        data: new_data,
                        proxy,
                    },
                );
            }
            PWObjectData::Port(new_data) => {
                let PWObject::Port {
                    data: old_data,
                    proxy,
                } = old_obj
                else {
                    warn!(target: "PWGraph::update", "Tried to update Port, but object of ID {id} is not a Port");
                    self.objects.insert(id, old_obj);
                    return false;
                };

                if new_data == old_data {
                    trace!(target: "PWGraph::update", "Tried to update Port ({id}), but it is unmodified");
                    self.objects.insert(
                        id,
                        PWObject::Port {
                            data: old_data,
                            proxy,
                        },
                    );
                    return false;
                }

                let PortData {
                    node_id: new_node_id,
                    direction: new_direction,
                    ..
                } = new_data;
                let PortData {
                    node_id: old_node_id,
                    direction: old_direction,
                    ..
                } = old_data;

                if new_node_id != old_node_id || new_direction != old_direction {
                    if let (Some(new_node_id), Some(new_direction)) = (new_node_id, new_direction) {
                        if let (Some(old_node_id), Some(old_direction)) =
                            (old_node_id, old_direction)
                        {
                            match old_direction {
                                Direction::Input => {
                                    self.get_node_input_ports(&old_node_id).remove(&id);
                                }
                                Direction::Output => {
                                    self.get_node_output_ports(&old_node_id).remove(&id);
                                }
                                _ => {}
                            }
                        }
                        match new_direction {
                            Direction::Input => {
                                self.get_node_input_ports(&new_node_id).insert(id);
                            }
                            Direction::Output => {
                                self.get_node_output_ports(&new_node_id).insert(id);
                            }
                            _ => {}
                        }
                    }
                }

                let new_data = PortData::join(old_data.clone(), new_data);
                debug!(target: "PWGraph::update", "Updated Port ({id}) {:?} -> {:?}", old_data, new_data);
                self.objects.insert(
                    id,
                    PWObject::Port {
                        data: new_data,
                        proxy,
                    },
                );
            }
            PWObjectData::Link(new_data) => {
                let PWObject::Link {
                    data: old_data,
                    proxy,
                } = old_obj
                else {
                    warn!(target: "PWGraph::update", "Tried to update Link, but object of ID {id} is not a Link");
                    self.objects.insert(id, old_obj);
                    return false;
                };

                if new_data == old_data {
                    trace!(target: "PWGraph::update", "Tried to update Link ({id}), but it is unmodified");
                    self.objects.insert(
                        id,
                        PWObject::Link {
                            data: old_data,
                            proxy,
                        },
                    );
                    return false;
                }

                let LinkData {
                    input_port: new_input_port,
                    output_port: new_output_port,
                    ..
                } = new_data;
                let LinkData {
                    input_port: old_input_port,
                    output_port: old_output_port,
                    ..
                } = old_data;

                if new_output_port != old_output_port {
                    if let Some(new_output_port) = new_output_port {
                        if let Some(old_output_port) = old_output_port {
                            self.get_links_from_port(&old_output_port).remove(&id);
                        }
                        self.get_links_from_port(&new_output_port).insert(id);
                    }
                }

                if new_input_port != old_input_port {
                    if let Some(new_input_port) = new_input_port {
                        if let Some(old_input_port) = old_input_port {
                            self.get_links_to_port(&old_input_port).remove(&id);
                        }
                        self.get_links_to_port(&new_input_port).insert(id);
                    }
                }

                let new_data = LinkData::join(old_data.clone(), new_data);
                debug!(target: "PWGraph::update", "Updated Link ({id}) {:?} -> {:?}", old_data, new_data);
                self.objects.insert(
                    id,
                    PWObject::Link {
                        data: new_data,
                        proxy,
                    },
                );
            }
        };

        true
    }

    pub fn remove(&mut self, id: Id) -> Option<PWObject> {
        trace!(target: "PWGraph::remove", "Called for object with ID {id}");
        let removed = self.objects.remove(&id);

        match removed {
            Some(PWObject::Node { ref data, .. }) => {
                let NodeData { media_class, .. } = data;
                if let Some(media_class) = media_class {
                    if media_class.contains("Sink") {
                        self.sinks.remove(&id);
                    }
                }
                debug!(target: "PWGraph::remove", "Removed Node ({id})");
            }
            Some(PWObject::Port { ref data, .. }) => {
                let PortData {
                    node_id, direction, ..
                } = data;
                if let (Some(node_id), Some(direction)) = (node_id, direction) {
                    match *direction {
                        Direction::Input => {
                            self.get_node_input_ports(node_id).remove(&id);
                        }
                        Direction::Output => {
                            self.get_node_output_ports(node_id).remove(&id);
                        }
                        _ => {}
                    };
                }
                debug!(target: "PWGraph::remove", "Removed Port ({id})");
            }
            Some(PWObject::Link { ref data, .. }) => {
                let LinkData {
                    input_port,
                    output_port,
                    ..
                } = data;
                if let Some(output_port) = output_port {
                    self.get_links_from_port(output_port).remove(&id);
                };

                if let Some(input_port) = input_port {
                    self.get_links_to_port(input_port).remove(&id);
                };
                debug!(target: "PWGraph::remove", "Removed Link ({id})");
            }
            None => {
                trace!(target: "PWGraph::remove", "Tried to remove inexistent object with ID {id}");
            }
        };

        removed
    }

    pub fn get(&self, id: &Id) -> Option<&PWObject> {
        self.objects.get(id)
    }

    pub fn get_links_to_port(&mut self, port: &Id) -> &mut HashSet<Id> {
        self.links_to_port.entry(*port).or_default()
    }

    pub fn get_links_from_port(&mut self, port: &Id) -> &mut HashSet<Id> {
        self.links_from_port.entry(*port).or_default()
    }

    pub fn get_node_input_ports(&mut self, node_id: &Id) -> &mut HashSet<Id> {
        self.node_input_ports.entry(*node_id).or_default()
    }

    pub fn get_node_output_ports(&mut self, node_id: &Id) -> &mut HashSet<Id> {
        self.node_output_ports.entry(*node_id).or_default()
    }

    pub fn get_sinks(&self) -> &HashSet<Id> {
        &self.sinks
    }

    pub fn get_active_sinks(&self) -> HashSet<&Id> {
        let mut active_sinks: HashSet<&Id> = HashSet::new();

        if self.get_sinks().is_empty() {
            warn!(target: "PWGraph::get_active_sinks", "List of sinks is empty");
        }

        for sink in self.get_sinks() {
            trace!(target: "PWgraph::get_active_sinks", "Starting transversal from Sink {sink}");
            if self.check_node_active(sink, &mut HashSet::new()) {
                active_sinks.insert(sink);
            }
        }

        active_sinks
    }

    fn check_node_active(&self, id: &Id, visited: &mut HashSet<Id>) -> bool {
        visited.insert(*id);

        trace!(target: "PWGraph::check_node_active", "Node {id}");
        match self.get(id) {
            Some(PWObject::Node { data, .. }) => {
                if NodeFilter::matches_any(&self.node_blacklist, data) {
                    return false;
                }
            }
            None => {
                warn!(target: "PWGraph::check_node_active", "While transversing graph, got invalid id {id}");
                return false;
            }
            _ => {
                warn!(target: "PWGraph::check_node_active", "While transversing graph expected Node, but got something else with id {id}");
                return false;
            }
        };

        let Some(node_input_ports) = self.node_input_ports.get(id) else {
            trace!(target: "PWGraph::check_node_active", "Node ({id}) has no input ports, assuming it is a client");
            return true;
        };

        if node_input_ports.is_empty() {
            trace!(target: "PWGraph::check_node_active", "Node ({id}) has no input ports, assuming it is a client");
            return true;
        };

        trace!(
            target: "PWGraph::check_node_active",
            "Transversing Graph: Node {id}: Node Input Ports: {}",
            node_input_ports.len()
        );

        let mut links_to_node: HashSet<(&Id, &Id)> = HashSet::new();
        for port in node_input_ports {
            let Some(PWObject::Port { .. }) = self.get(port) else {
                warn!(target: "PWGraph::check_node_active", "While transversing graph, expected Port, got something else with id {port}");
                continue;
            };
            trace!("Transversing Graph: Node {id}: Input Port {port}");
            let Some(links) = self.links_to_port.get(port) else {
                trace!(target: "PWGraph::check_node_active", "Transversing Graph: Node {id}: No links to Input Port {port}");
                continue;
            };
            if links.is_empty() {
                trace!(target: "PWGraph::check_node_active", "Transversing Graph: Node {id}: No links to Input Port {port}");
                continue;
            };
            trace!(
                target: "PWGraph::check_node_active",
                "Transversing Graph: Node {id}: links to Input Port {port}: {}",
                links.len()
            );
            for link in links {
                let Some(PWObject::Link { data, .. }) = self.get(link) else {
                    warn!(target: "PWGraph::check_node_active", "While transversing graph, expected Link, got something else with id {link}");
                    continue;
                };
                let LinkData {
                    output_port,
                    active,
                    ..
                } = data;

                if let Some(active) = active {
                    if !active {
                        continue;
                    }
                } else {
                    continue;
                }

                let Some(output_port) = output_port else {
                    warn!(target: "PWGraph::check_node_active", "Link ({link}) is missing output_port");
                    continue;
                };

                links_to_node.insert((&link, &output_port));
            }
        }

        if links_to_node.is_empty() {
            trace!(target: "PWGraph::check_node_active", "Transversing Graph: Node {id}: No Active Links to node");
            return false;
        };
        trace!(target: "PWGraph::check_node_active", "Transversing Graph: Node {id}: Active Links to node: {}", links_to_node.len());

        for (_, input_port) in links_to_node {
            let Some(PWObject::Port { data, .. }) = self.get(input_port) else {
                warn!(target: "PWGraph::check_node_active", "While transversing graph, expected Port, got something else with id {input_port}");
                continue;
            };
            let PortData { node_id, .. } = data;

            let Some(node_id) = node_id else {
                warn!(target: "PWGraph::check_node_active", "Port ({input_port}) is missing node_id");
                continue;
            };

            if !visited.contains(node_id) && self.check_node_active(node_id, visited) {
                return true;
            }
        }

        false
    }
}
