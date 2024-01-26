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

//! Module responsible to represent and treat the PipeWire Graph, in the context of this app,
//! composed of [PWObject]s, that can be Nodes, Links or Ports.

use std::collections::{HashMap, HashSet};

use log::{debug, trace, warn};
use pipewire::spa::Direction;

pub mod filter;
use filter::{Filter, NodeFilter, SinkFilter};

pub mod object;
use object::{Id, LinkData, NodeData, PWObject, PWObjectData, PortData};

/// Struct that represents the [pipewire] graph.
///
/// Tracked objects are store in a [HashMap] with its id used as key
///
/// Fast access to links attached to ports and the port's nodes are also kept in maps.
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
    /// Builds a new [PWGraph]
    ///
    /// The vectors of [SinkFilter]s and [NodeFilter]s are defined by the user and, thus, are
    /// passed as arguments.
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

    /// Inserts a new object into the Graph.
    ///
    /// Currently ID conflicts are not treated.
    pub fn insert(&mut self, id: Id, obj: PWObject) {
        match obj {
            PWObject::Node { ref data, .. } => {
                let NodeData {
                    ref media_class, ..
                } = data;
                debug!(target: "PWGraph::insert", "Node ({id}) '{}'; {:?}", data.get_name().unwrap_or_default(), data);
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
                            self.node_input_ports
                                .entry(*node_id)
                                .or_default()
                                .insert(id);
                        }
                        Direction::Output => {
                            debug!(target: "PWGraph::insert", "Port ({id}) as Node {node_id} Output; {:?}", data);
                            self.node_output_ports
                                .entry(*node_id)
                                .or_default()
                                .insert(id);
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
                    self.links_from_port
                        .entry(*output_port)
                        .or_default()
                        .insert(id);
                };

                if let Some(input_port) = input_port {
                    debug!(target: "PWGraph::insert", "Link ({id}) with input_port {input_port}");
                    self.links_to_port
                        .entry(*input_port)
                        .or_default()
                        .insert(id);
                };
            }
        }

        self.objects.insert(id, obj);
    }

    /// Updates an object data
    pub fn update(&mut self, id: Id, new_data: PWObjectData) -> bool {
        trace!(target: "PWGraph::update", "Called for object with ID {id}");
        let Some(obj) = self.objects.get_mut(&id) else {
            warn!(target: "PWGraph::update", "Tried to update inexistent object with ID {id}");
            return false;
        };

        match new_data {
            PWObjectData::Node(new_data) => {
                let PWObject::Node { ref mut data, .. } = obj else {
                    warn!(target: "PWGraph::update", "Tried to update Node, but object of ID {id} is not a Node");
                    return false;
                };

                let NodeData {
                    media_class: ref new_media_class,
                    ..
                } = new_data;

                let NodeData {
                    ref media_class, ..
                } = data;

                if media_class != new_media_class {
                    if let Some(new_media_class) = new_media_class {
                        if let Some(media_class) = media_class {
                            if media_class.contains("Sink") {
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

                debug!(target: "PWGraph::update", "Updated Node ({id}) from {:?}", data);
                let was_updated = data.update(new_data);
                debug!(target: "PWGraph::update", "Updated Node ({id}) to {:?}", data);
                was_updated
            }
            PWObjectData::Port(new_data) => {
                let PWObject::Port { ref mut data, .. } = obj else {
                    warn!(target: "PWGraph::update", "Tried to update Port, but object of ID {id} is not a Port");
                    return false;
                };

                let PortData {
                    node_id: ref new_node_id,
                    direction: ref new_direction,
                    ..
                } = new_data;
                let PortData {
                    ref node_id,
                    ref direction,
                    ..
                } = data;

                if node_id != new_node_id || direction != new_direction {
                    if let (Some(new_node_id), Some(new_direction)) = (new_node_id, new_direction) {
                        if let (Some(node_id), Some(direction)) = (node_id, direction) {
                            match *direction {
                                Direction::Input => {
                                    self.node_input_ports
                                        .entry(*node_id)
                                        .or_default()
                                        .remove(&id);
                                }
                                Direction::Output => {
                                    self.node_output_ports
                                        .entry(*node_id)
                                        .or_default()
                                        .remove(&id);
                                }
                                _ => {}
                            }
                        }
                        match *new_direction {
                            Direction::Input => {
                                self.node_input_ports
                                    .entry(*new_node_id)
                                    .or_default()
                                    .insert(id);
                            }
                            Direction::Output => {
                                self.node_output_ports
                                    .entry(*new_node_id)
                                    .or_default()
                                    .insert(id);
                            }
                            _ => {}
                        }
                    }
                }

                debug!(target: "PWGraph::update", "Updated Port ({id}) from {:?}", data);
                let was_updated = data.update(new_data);
                debug!(target: "PWGraph::update", "Updated Port ({id}) to {:?}", data);
                was_updated
            }
            PWObjectData::Link(new_data) => {
                let PWObject::Link { ref mut data, .. } = obj else {
                    warn!(target: "PWGraph::update", "Tried to update Link, but object of ID {id} is not a Link");
                    return false;
                };

                let LinkData {
                    input_port: ref new_input_port,
                    output_port: ref new_output_port,
                    ..
                } = new_data;
                let LinkData {
                    ref input_port,
                    ref output_port,
                    ..
                } = data;

                if output_port != new_output_port {
                    if let Some(new_output_port) = new_output_port {
                        if let Some(output_port) = output_port {
                            self.links_from_port
                                .entry(*output_port)
                                .or_default()
                                .remove(&id);
                        }
                        self.links_from_port
                            .entry(*new_output_port)
                            .or_default()
                            .insert(id);
                    }
                }

                if input_port != new_input_port {
                    if let Some(new_input_port) = new_input_port {
                        if let Some(input_port) = input_port {
                            self.links_to_port
                                .entry(*input_port)
                                .or_default()
                                .remove(&id);
                        }
                        self.links_to_port
                            .entry(*new_input_port)
                            .or_default()
                            .insert(id);
                    }
                }

                debug!(target: "PWGraph::update", "Updated Link ({id}) from {:?}", data);
                let was_updated = data.update(new_data);
                debug!(target: "PWGraph::update", "Updated Link ({id}) to {:?}", data);
                was_updated
            }
        }
    }

    /// Remove an object from the graph and cleans up references to it.
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
                            self.node_input_ports
                                .entry(*node_id)
                                .or_default()
                                .remove(&id);
                        }
                        Direction::Output => {
                            self.node_output_ports
                                .entry(*node_id)
                                .or_default()
                                .remove(&id);
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
                    self.links_from_port
                        .entry(*output_port)
                        .or_default()
                        .remove(&id);
                };

                if let Some(input_port) = input_port {
                    self.links_to_port
                        .entry(*input_port)
                        .or_default()
                        .remove(&id);
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

    /// Looks for sinks with active links to tracked nodes.
    ///
    /// If a sink_whitelist is passed to the graph, only sinks that match it will be treated.
    pub fn get_active_sinks(&self) -> HashSet<&Id> {
        let mut active_sinks: HashSet<&Id> = HashSet::new();

        if self.sinks.is_empty() {
            warn!(target: "PWGraph::get_active_sinks", "List of sinks is empty");
        }

        for sink in &self.sinks {
            trace!(target: "PWgraph::get_active_sinks", "Starting transversal from Sink {sink}");
            if self.check_node_active(sink, &mut HashSet::new()) {
                active_sinks.insert(sink);
            }
        }

        active_sinks
    }

    /// Transverses the Graphs in a manner similar to a DFS algorithm, looking for active
    /// connections from sinks to nodes.
    ///
    /// If a node_blacklist was passed, nodes that match it will be ignored.
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
