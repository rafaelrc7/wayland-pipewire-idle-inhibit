// Copyright (C) 2023-2024  Rafael Carvalho <contact@rafaelrc.com>

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

use std::{
    any::Any,
    cell::RefCell,
    marker::Send,
    rc::Rc,
    sync::mpsc,
    thread::{self, JoinHandle},
};

use pipewire::{
    keys,
    link::{Link, LinkChangeMask, LinkInfo, LinkListener, LinkState},
    node::{Node, NodeInfo, NodeListener},
    port::{Port, PortInfo, PortListener},
    prelude::ReadableDict,
    registry::{GlobalObject, Registry},
    spa::{Direction, ForeignDict},
    types::ObjectType,
    Context, MainLoop,
};

use log::debug;

pub mod graph;
use graph::{Id, LinkData, NodeData, PWGraph, PWObject, PWObjectData, PortData, Proxy};

#[derive(Debug)]
pub enum PWMsg {
    Terminate,
    GraphUpdated,
}

#[derive(Debug)]
pub enum PWEvent {
    GraphUpdated,
    InhibitIdleState(bool),
}

pub struct PWThread {
    pw_thread: JoinHandle<()>,
    pw_event_sender: pipewire::channel::Sender<PWMsg>,
}

impl PWThread {
    pub fn new<Msg: From<PWEvent> + Send + 'static>(pw_event_listener: mpsc::Sender<Msg>) -> Self {
        let (pw_event_sender, pw_event_queue) = pipewire::channel::channel();

        let pw_thread = thread::spawn({
            let pw_event_listener = pw_event_listener.clone();
            move || pw_thread(pw_event_listener, pw_event_queue)
        });

        PWThread {
            pw_thread,
            pw_event_sender,
        }
    }

    pub fn join(self) -> Result<(), Box<dyn Any + Send>> {
        let PWThread { pw_thread, .. } = self;
        pw_thread.join()
    }

    pub fn send(&self, msg: PWMsg) -> Result<(), PWMsg> {
        self.pw_event_sender.send(msg)
    }
}

fn pw_thread<Msg: From<PWEvent> + 'static>(
    pw_event_listener: mpsc::Sender<Msg>,
    pw_event_queue: pipewire::channel::Receiver<PWMsg>,
) {
    pipewire::init();
    let mainloop = MainLoop::new().expect("Failed to create mainloop.");

    let graph = Rc::new(RefCell::new(PWGraph::new()));

    let context = Rc::new(Context::new(&mainloop).expect("Failed to create context."));
    let core = Rc::new(context.connect(None).expect("Failed to get core."));
    let registry = Rc::new(core.get_registry().expect("Failed to get registry"));

    let _listener = {
        registry
            .add_listener_local()
            .global({
                let registry = Rc::clone(&registry);
                let graph = Rc::clone(&graph);
                let pw_event_listener = pw_event_listener.clone();

                move |global| {
                    let registry = Rc::clone(&registry);
                    let graph = Rc::clone(&graph);

                    match global.type_ {
                        ObjectType::Node => {
                            registry_global_node(global, registry, graph, pw_event_listener.clone())
                        }
                        ObjectType::Port => {
                            registry_global_port(global, registry, graph, pw_event_listener.clone())
                        }
                        ObjectType::Link => {
                            registry_global_link(global, registry, graph, pw_event_listener.clone())
                        }
                        _ => {}
                    }
                }
            })
            .global_remove({
                let graph = Rc::clone(&graph);
                let pw_event_listener = pw_event_listener.clone();

                move |id| {
                    registry_global_remove(id, Rc::clone(&graph), pw_event_listener.clone());
                }
            })
            .register()
    };

    let _receiver = pw_event_queue.attach(&mainloop, {
        let mainloop = mainloop.clone();

        move |signal: PWMsg| match signal {
            PWMsg::Terminate => mainloop.quit(),
            PWMsg::GraphUpdated => {
                let should_inhibit_idle = !graph.borrow_mut().get_active_sinks().is_empty();
                pw_event_listener
                    .send(Msg::from(PWEvent::InhibitIdleState(should_inhibit_idle)))
                    .unwrap();
            }
        }
    });

    mainloop.run();
}

fn registry_global_node<Msg: From<PWEvent> + 'static>(
    node: &GlobalObject<ForeignDict>,
    registry: Rc<Registry>,
    graph: Rc<RefCell<PWGraph>>,
    pw_event_listener: mpsc::Sender<Msg>,
) {
    let id = node.id;
    let props = node
        .props
        .as_ref()
        .expect("Node object is missing properties");
    let name = props.get(&keys::NODE_NAME).map(|s| s.to_string());
    let app_name = props.get(&keys::APP_NAME).map(|s| s.to_string());
    let description = props.get(&keys::NODE_DESCRIPTION).map(|s| s.to_string());
    let nick = props.get(&keys::NODE_NICK).map(|s| s.to_string());
    let media_class = props.get(&keys::MEDIA_CLASS).map(|s| s.to_string());
    let media_role = props.get(&keys::MEDIA_ROLE).map(|s| s.to_string());
    let media_software = props.get(&keys::MEDIA_SOFTWARE).map(|s| s.to_string());

    let proxy: Node = registry.bind(node).expect("Failed to bind Node Proxy");
    let listener: NodeListener = proxy
        .add_listener_local()
        .info({
            let graph = Rc::clone(&graph);
            let pw_event_listener = pw_event_listener.clone();
            move |info| node_info(info, Rc::clone(&graph), pw_event_listener.clone())
        })
        .register();

    let data = NodeData {
        name,
        app_name,
        description,
        nick,
        media_class,
        media_role,
        media_software,
    };
    graph.borrow_mut().insert(
        id,
        PWObject::Node {
            data,
            proxy: Proxy { proxy, listener },
        },
    );

    pw_event_listener
        .send(Msg::from(PWEvent::GraphUpdated))
        .unwrap();
}

fn node_info<Msg: From<PWEvent>>(
    info: &NodeInfo,
    graph: Rc<RefCell<PWGraph>>,
    pw_event_listener: mpsc::Sender<Msg>,
) {
    let id = info.id();
    debug!("Event Node Info id:{id}");

    let props = info.props().expect("NodeInfo object is missing properties");
    let name = props.get(&keys::NODE_NAME).map(|s| s.to_string());
    let app_name = props.get(&keys::APP_NAME).map(|s| s.to_string());
    let description = props.get(&keys::NODE_DESCRIPTION).map(|s| s.to_string());
    let nick = props.get(&keys::NODE_NICK).map(|s| s.to_string());
    let media_class = props.get(&keys::MEDIA_CLASS).map(|s| s.to_string());
    let media_role = props.get(&keys::MEDIA_ROLE).map(|s| s.to_string());
    let media_software = props.get(&keys::MEDIA_SOFTWARE).map(|s| s.to_string());

    let new_data = NodeData {
        name,
        app_name,
        description,
        nick,
        media_class,
        media_role,
        media_software,
    };
    if graph.borrow_mut().update(id, PWObjectData::Node(new_data)) {
        pw_event_listener
            .send(Msg::from(PWEvent::GraphUpdated))
            .unwrap();
    }
}

fn direction_from_string(direction: &str) -> Option<Direction> {
    match direction {
        "out" => Some(Direction::Output),
        "in" => Some(Direction::Input),
        _ => None,
    }
}

fn registry_global_port<Msg: From<PWEvent> + 'static>(
    port: &GlobalObject<ForeignDict>,
    registry: Rc<Registry>,
    graph: Rc<RefCell<PWGraph>>,
    pw_event_listener: mpsc::Sender<Msg>,
) {
    let id = port.id;

    let props = port
        .props
        .as_ref()
        .expect("Port object is missing properties");
    let name = props.get(&keys::PORT_NAME).map(|s| s.to_string());
    let node_id: Option<Id> = props.get(&keys::NODE_ID).map(|s| s.parse().ok()).flatten();
    let direction = props
        .get(&keys::PORT_DIRECTION)
        .map(|s| direction_from_string(s))
        .flatten();
    let is_terminal: Option<bool> = props
        .get(&keys::PORT_TERMINAL)
        .map(|s| s.parse().ok())
        .flatten();

    let proxy: Port = registry.bind(port).expect("Failed to bind Port Proxy");
    let listener: PortListener = proxy
        .add_listener_local()
        .info({
            let graph = Rc::clone(&graph);
            let pw_event_listener = pw_event_listener.clone();
            move |info| port_info(info, Rc::clone(&graph), pw_event_listener.clone())
        })
        .param(move |_, _param_id, _, _, _param| {}) // TODO
        .register();

    let data = PortData {
        name,
        node_id,
        direction,
        is_terminal,
    };
    graph.borrow_mut().insert(
        id,
        PWObject::Port {
            data,
            proxy: Proxy { proxy, listener },
        },
    );

    pw_event_listener
        .send(Msg::from(PWEvent::GraphUpdated))
        .unwrap();
}

fn port_info<Msg: From<PWEvent>>(
    info: &PortInfo,
    graph: Rc<RefCell<PWGraph>>,
    pw_event_listener: mpsc::Sender<Msg>,
) {
    let id = info.id();
    debug!("Event Port Info id:{id}");

    let props = info.props().expect("PortInfo object is missing properties");
    let name = props.get(&keys::PORT_NAME).map(|s| s.to_string());
    let node_id: Option<Id> = props.get(&keys::NODE_ID).map(|s| s.parse().ok()).flatten();
    let direction = props
        .get(&keys::PORT_DIRECTION)
        .map(|s| direction_from_string(s))
        .flatten();
    let is_terminal: Option<bool> = props
        .get(&keys::PORT_TERMINAL)
        .map(|s| s.parse().ok())
        .flatten();

    let new_data = PortData {
        name,
        node_id,
        direction,
        is_terminal,
    };
    if graph.borrow_mut().update(id, PWObjectData::Port(new_data)) {
        pw_event_listener
            .send(Msg::from(PWEvent::GraphUpdated))
            .unwrap();
    }
}

fn registry_global_link<Msg: From<PWEvent> + 'static>(
    link: &GlobalObject<ForeignDict>,
    registry: Rc<Registry>,
    graph: Rc<RefCell<PWGraph>>,
    pw_event_listener: mpsc::Sender<Msg>,
) {
    let id = link.id;

    let props = link
        .props
        .as_ref()
        .expect("Port object is missing properties");

    let input_port: Option<Id> = props
        .get(&keys::LINK_INPUT_PORT)
        .map(|s| s.parse().ok())
        .flatten();
    let output_port: Option<Id> = props
        .get(&keys::LINK_OUTPUT_PORT)
        .map(|s| s.parse().ok())
        .flatten();
    let active = Some(false);

    let proxy: Link = registry.bind(link).expect("Failed to bind Link Proxy");
    let listener: LinkListener = proxy
        .add_listener_local()
        .info({
            let graph = Rc::clone(&graph);
            let pw_event_listener = pw_event_listener.clone();
            move |info| link_info(info, Rc::clone(&graph), pw_event_listener.clone())
        })
        .register();

    let data = LinkData {
        input_port,
        output_port,
        active,
    };
    graph.borrow_mut().insert(
        id,
        PWObject::Link {
            data,
            proxy: Proxy { proxy, listener },
        },
    );

    pw_event_listener
        .send(Msg::from(PWEvent::GraphUpdated))
        .unwrap();
}

fn link_info<Msg: From<PWEvent>>(
    info: &LinkInfo,
    graph: Rc<RefCell<PWGraph>>,
    pw_event_listener: mpsc::Sender<Msg>,
) {
    let id = info.id();
    debug!("Event Link Info id:{id}");

    let props = info.props().expect("LinkInfo object is missing properties");
    let input_port: Option<Id> = props
        .get(&keys::LINK_INPUT_PORT)
        .map(|s| s.parse().ok())
        .flatten();
    let output_port: Option<Id> = props
        .get(&keys::LINK_OUTPUT_PORT)
        .map(|s| s.parse().ok())
        .flatten();

    let active = if info.change_mask().contains(LinkChangeMask::STATE) {
        Some(matches!(info.state(), LinkState::Active))
    } else {
        None
    };

    let new_data = LinkData {
        input_port,
        output_port,
        active,
    };
    if graph.borrow_mut().update(id, PWObjectData::Link(new_data)) {
        pw_event_listener
            .send(Msg::from(PWEvent::GraphUpdated))
            .unwrap();
    }
}

fn registry_global_remove<Msg: From<PWEvent>>(
    id: Id,
    graph: Rc<RefCell<PWGraph>>,
    pw_event_listener: mpsc::Sender<Msg>,
) {
    debug!("Event Registry Global Remove Object id: {id}");
    graph.borrow_mut().remove(id);

    pw_event_listener
        .send(Msg::from(PWEvent::GraphUpdated))
        .unwrap();
}
