use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::{rc::Rc, collections::HashMap};

use pipewire::link::{Link, LinkListener, LinkInfo, LinkChangeMask, LinkState};
use pipewire::loop_::EventSource;
use pipewire::main_loop::MainLoop;
use pipewire::node::{NodeListener, NodeInfo};
use pipewire::port::{Port, PortListener, PortInfo};
use pipewire::proxy::{Listener, ProxyT};
use pipewire::registry::{GlobalObject, Registry};
use pipewire::spa::utils::Direction;
use pipewire::spa::utils::dict::DictRef;
use pipewire::{init, node::Node, types::ObjectType, keys};
use pipewire::context::Context;
use log::{log, trace, debug, info, warn, error};

type Id = u32;

struct Proxy<TProxy: ProxyT, TListener: Listener> {
    proxy: TProxy,
    listener: TListener
}

enum PWObject {
    Node {
        name: String,
        media_class: String,
        proxy: Proxy<Node, NodeListener>,
    },
    Port {
        name: String,
        node_id: Id,
        direction: Direction,
        is_terminal: bool,
        proxy: Proxy<Port, PortListener>,
    },
    Link {
        input_port: Id,
        output_port: Id,
        active: bool,
        proxy: Proxy<Link, LinkListener>,
    },
}

#[derive(Default)]
struct IdleState {
    idle: bool,
}

impl IdleState {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Default)]
struct PWGraph {
    objects: HashMap<Id, PWObject>,
    sinks: HashSet<Id>,
    links_to_port: HashMap<Id, HashSet<Id>>,
    links_from_port: HashMap<Id, HashSet<Id>>,
    node_ports: HashMap<Id, HashSet<Id>>,
}

impl PWGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, id: Id, obj: PWObject) {
        match obj {
            PWObject::Link { input_port, output_port, .. } => {
                let mut links_to_port: HashSet<Id> = match self.get_links_to_port(&output_port) {
                    Some(s) => s.to_owned(),
                    None => HashSet::new(),
                };
                links_to_port.insert(id);
                self.links_to_port.insert(output_port, links_to_port);

                let mut links_from_port: HashSet<Id> = match self.get_links_from_port(&input_port) {
                    Some(v) => v.to_owned(),
                    None => HashSet::new(),
                };
                links_from_port.insert(id);
                self.links_from_port.insert(input_port, links_from_port);
            },
            PWObject::Port { node_id, .. } => {
                let mut node_ports: HashSet<Id> = match self.get_node_ports(&node_id) {
                    Some(s) => s.to_owned(),
                    None => HashSet::new(),
                };
                node_ports.insert(id);
                self.node_ports.insert(node_id, node_ports);
            },
            PWObject::Node { ref media_class, .. } => {
                if media_class.contains("Sink") {
                    self.sinks.insert(id);
                }
            },
        }

        self.objects.insert(id, obj);
    }

    pub fn get(&self, id: &Id) -> Option<&PWObject> {
        self.objects.get(id)
    }

    pub fn get_links_to_port(&self, output_port: &Id) -> Option<&HashSet<Id>> {
        self.links_to_port.get(output_port)
    }

    pub fn get_links_from_port(&self, input_port: &Id) -> Option<&HashSet<Id>> {
        self.links_from_port.get(input_port)
    }

    pub fn get_node_ports(&self, node_id: &Id) -> Option<&HashSet<Id>> {
        self.node_ports.get(node_id)
    }

    pub fn get_sinks(&self) -> &HashSet<Id> {
        &self.sinks
    }

    pub fn map_object<F: Fn(PWObject) -> PWObject>(&mut self, id: Id, update: F) {
        if let Some(obj) = self.objects.remove(&id) {
            self.objects.insert(id, update(obj));
        };
    }

    pub fn remove(&mut self, id: Id) -> Option<PWObject> {
        let removed = self.objects.remove(&id);

        match removed {
            Some(PWObject::Link { input_port, output_port, .. }) => {
                self.links_from_port.remove(&input_port);
                self.links_to_port.remove(&output_port);
            },
            Some(PWObject::Port { node_id, .. }) => {
                if let Some(node_ports) = self.get_node_ports(&node_id) {
                    let mut node_ports = node_ports.to_owned();
                    node_ports.remove(&id);
                    if node_ports.len() > 0 {
                        self.node_ports.insert(node_id, node_ports);
                    } else {
                        self.node_ports.remove(&node_id);
                    }
                }
            }
            _ => {},
        };

        removed
    }

    pub fn get_active_sinks(&self) -> HashSet<&Id> {
        let mut active_sinks: HashSet<&Id> = HashSet::new();

        for sink in &self.sinks {
            if self.check_node_active(sink, &mut HashSet::new()) {
                active_sinks.insert(sink);
            }
        }

        active_sinks
    }

    fn check_node_active(&self, id: &Id, visited: &mut HashSet<Id>) -> bool {
        visited.insert(id.clone());

        match self.get(id) {
            Some(PWObject::Node { .. }) => {
                // TODO: Specific Node tests
            },
            None => {
                error!("While transversing graph, got invalid id {id}");
                return false
            },
            _ => {
                warn!("While transversing graph, got unexpected object with id {id}");
                return false
            },
        };

        let Some(node_ports) = self.node_ports.get(id) else {
            return false;
        };

        let mut has_no_input_ports = true;
        let mut links_to_node:HashSet<(&Id, &Id)> = HashSet::new();
        for port in node_ports {
            let Some(PWObject::Port { ref direction, .. }) = self.get(port) else {
                error!("While transversing graph, expected Port, got something else with id {port}");
                continue;
            };
            if !(*direction == Direction::Input) {
                continue;
            } else {
                has_no_input_ports = false;
            }
            let Some(links) = self.get_links_to_port(&port) else {
                continue;
            };
            for link in links {
                let Some(PWObject::Link { input_port, active, .. }) = self.get(link) else {
                    error!("While transversing graph, expected Link, got something else with id {link}");
                    continue;
                };
                if *active {
                    links_to_node.insert((&link, &input_port));
                }
            }
        };

        if links_to_node.len() == 0 {
            return has_no_input_ports; // Has no input ports, thus is an active client
        };

        for (_, input_port) in links_to_node {
            let Some(PWObject::Port { node_id, .. }) = self.get(&input_port) else {
                error!("While transversing graph, expected Port, got something else with id {input_port}");
                continue;
            };
            if !visited.contains(node_id) {
                if self.check_node_active(node_id, visited) {
                    return true;
                }
            }
        };

        return false;
    }
}

fn get_node_name(props: &DictRef) -> Option<&str> {
    props.get(&keys::APP_NAME)
         .or_else(|| props.get(&keys::NODE_DESCRIPTION))
         .or_else(|| props.get(&keys::NODE_NICK))
         .or_else(|| props.get(&keys::NODE_NAME))
}

fn registry_global_node(node: &GlobalObject<&DictRef>, registry: Rc<Registry>, graph: Arc<Mutex<PWGraph>>, update_event: Rc<EventSource>) {
    let id = node.id;
    let props = node.props.as_ref().expect("Node object is missing properties");
    let name = get_node_name(props).unwrap_or("UNDEFINED").to_string();
    let media_class = props.get(&keys::MEDIA_CLASS).unwrap_or("UNDEFINED").to_string();

    let proxy: Node = registry.bind(node).expect("Failed to bind Node Proxy");
    let listener: NodeListener = proxy.add_listener_local()
                                      .info({
                                          let graph = Arc::clone(&graph);
                                          move |info| node_info(info, Arc::clone(&graph))
                                      })
                                      .register();

    info!("Event Registry Global Created Node id: {id}\tname: {name}\tmedia class: {media_class}");
    let mut graph = graph.lock().expect("Failed to lock graph mutex");
    graph.insert(id, PWObject::Node { name, media_class, proxy: Proxy {proxy, listener} });
}

fn node_info(info: &NodeInfo, graph: Arc<Mutex<PWGraph>>) {
    let id = info.id();
    let mut graph = graph.lock().expect("Failed to lock graph mutex");

    let Some(PWObject::Node { name, media_class, .. }) = graph.get(&id) else {
        error!("'info' event fired for unknown Node with id {id}");
        return;
    };

    let name = name.clone();
    let media_class = media_class.clone();

    info!("Event Node Info id:{id}");
    let props = info.props().expect("NodeInfo object is missing properties");

    if let Some(new_name) = get_node_name(props) {
        if new_name != name {
            info!("Updated Node id: {id} name: {name} -> {new_name}");
            graph.map_object(id, move |obj: PWObject| {
                match obj {
                    PWObject::Node { media_class, proxy, .. } => PWObject::Node {name: new_name.to_string(), media_class, proxy },
                    o => o,
                }
            });
        }
    };

    if let Some(new_media_class) = props.get(&keys::MEDIA_CLASS) {
        if new_media_class != media_class {
            info!("Updated Node id: {id} media_class: {media_class} -> {new_media_class}");
            graph.map_object(id, move |obj: PWObject| {
                match obj {
                    PWObject::Node { name, proxy, .. } => PWObject::Node {name, media_class: new_media_class.to_string(), proxy },
                    o => o,
                }
            });
        }
    };
}

fn direction_from_string(direction: &str) -> Direction {
    match direction {
        "out" => Direction::Output,
        "in" => Direction::Input,
        _ => panic!("Port direction is invalid")
    }
}

fn registry_global_port(port: &GlobalObject<&DictRef>, registry: Rc<Registry>, graph: Arc<Mutex<PWGraph>>, update_event: Rc<EventSource>) {
    let id = port.id;

    let props = port.props.as_ref().expect("Port object is missing properties");
    let name = props.get(&keys::PORT_NAME).unwrap_or("UNDEFINED").to_string();
    let node_id: Id = props.get(&keys::NODE_ID)
                           .expect("Port properties is missing Node ID")
                           .parse()
                           .expect("Failed to parse Port Node ID");
    let direction: Direction = direction_from_string(props.get(&keys::PORT_DIRECTION).expect("Port properties is missing Direction"));
    let is_terminal: bool = match props.get(&keys::PORT_TERMINAL) {
        None => false,
        Some(terminal) => terminal.parse().expect("Failed to parse Port terminal"),
    };

    let proxy: Port = registry.bind(port).expect("Failed to bind Port Proxy");
    let listener: PortListener = proxy.add_listener_local()
                                      .info({
                                          let graph = Arc::clone(&graph);
                                          move |info| port_info(info, Arc::clone(&graph))
                                      })
                                      .param(move |_, _param_id, _, _, _param| {} ) // TODO
                                      .register();

    info!("Event Registry Global Created Port ID: {id}\tNode ID: {node_id}\tName: {name}\tDirection: {:?}\tTerminal: {is_terminal}", direction);
    let mut graph = graph.lock().expect("Failed to lock graph mutex");
    graph.insert(id, PWObject::Port { name, node_id, direction, is_terminal, proxy: Proxy {proxy, listener} });
}

fn port_info(info: &PortInfo, graph: Arc<Mutex<PWGraph>>) {
    let id = info.id();
    let mut graph = graph.lock().expect("Failed to lock graph mutex");

    let Some(PWObject::Port { name, node_id, direction, is_terminal, .. }) = graph.get(&id) else {
        error!("'info' event fired for unknown Port with id {id}");
        return;
    };

    let name = name.clone();
    let node_id = node_id.clone();
    let direction = direction.clone();
    let is_terminal = is_terminal.clone();

    info!("Event Port Info id:{id}");
    let props = info.props().expect("PortInfo object is missing properties");

    if let Some(new_name) = props.get(&keys::PORT_NAME) {
        if new_name != name {
            info!("Updated Port id: {id} name: {name} -> {new_name}");
            graph.map_object(id, move |obj: PWObject| {
                match obj {
                    PWObject::Port { node_id, direction, is_terminal, proxy, .. } => PWObject::Port {name: new_name.to_string(), node_id, direction, is_terminal, proxy },
                    o => o,
                }
            });
        }
    };

    if let Some(new_node_id) = props.get(&keys::NODE_ID) {
        let new_node_id: Id = new_node_id.parse().expect("Failed to parse Port Node ID");
        if new_node_id != node_id {
            info!("Updated Port id: {id} Node ID: {node_id} -> {new_node_id}");
            graph.map_object(id, move |obj: PWObject| {
                match obj {
                    PWObject::Port { name, direction, is_terminal, proxy, .. } => PWObject::Port {name, node_id: new_node_id, direction, is_terminal, proxy },
                    o => o,
                }
            });
        }
    };

    if let Some(new_direction) = props.get(&keys::PORT_DIRECTION) {
        let new_direction = direction_from_string(new_direction);
        if new_direction != direction {
            info!("Updated Port id: {id} direction: {:?} -> {:?}", direction, new_direction);
            graph.map_object(id, move |obj: PWObject| {
                match obj {
                    PWObject::Port { name, node_id, is_terminal, proxy, .. } => PWObject::Port {name, node_id, direction: new_direction, is_terminal, proxy },
                    o => o,
                }
            });
        }
    };

    if let Some(new_is_terminal) = props.get(&keys::PORT_TERMINAL) {
        let new_is_terminal: bool = new_is_terminal.parse().expect("Failed to parse Port Terminal");
        if new_is_terminal != is_terminal {
            info!("Updated Port id: {id} Terminal: {is_terminal} -> {new_is_terminal}");
            graph.map_object(id, move |obj: PWObject| {
                match obj {
                    PWObject::Port { name, node_id, direction, proxy, .. } => PWObject::Port {name, node_id, direction, is_terminal: new_is_terminal, proxy },
                    o => o,
                }
            });
        }
    };
}

fn registry_global_link(link: &GlobalObject<&DictRef>, registry: Rc<Registry>, graph: Arc<Mutex<PWGraph>>, update_event: Rc<EventSource>) {
    let id = link.id;

    let props = link.props.as_ref().expect("Port object is missing properties");

    let input_port: Id = props.get(&keys::LINK_INPUT_PORT)
                              .expect("Link missing input port property")
                              .parse()
                              .expect("Failed to parse Link input port");
    let output_port: Id = props.get(&keys::LINK_OUTPUT_PORT)
                               .expect("Link missing output port property")
                               .parse()
                               .expect("Failed to parse Link output port");

    let proxy: Link = registry.bind(link).expect("Failed to bind Link Proxy");
    let listener: LinkListener = proxy.add_listener_local()
                                      .info({
                                          let graph = Arc::clone(&graph);
                                          move |info| link_info(info, Arc::clone(&graph))
                                      })
                                      .register();

    info!("Event Registry Global Created Link ID: {id}\tInput Port ID: {input_port}\tOutput Port ID: {output_port}");
    let mut graph = graph.lock().expect("Failed to lock graph mutex");
    graph.insert(id, PWObject::Link { input_port, output_port, active: false, proxy: Proxy {proxy, listener} });
}

fn link_info(info: &LinkInfo, graph: Arc<Mutex<PWGraph>>) {
    let id = info.id();
    let mut graph = graph.lock().expect("Failed to lock graph mutex");

    let Some(PWObject::Link { input_port, output_port, active, .. }) = graph.get(&id) else {
        error!("'info' event fired for unknown Link with id {id}");
        return;
    };

    let input_port = input_port.clone();
    let output_port = output_port.clone();
    let active = active.clone();

    info!("Event Link Info id:{id}");
    let props = info.props().expect("LinkInfo object is missing properties");

    if let Some(new_input_port) = props.get(&keys::LINK_INPUT_PORT) {
        let new_input_port: Id = new_input_port.parse().expect("Failed to parse Link Input Port");
        if new_input_port != input_port {
            info!("Updated Link id: {id} Input Port: {input_port} -> {new_input_port}");
            graph.map_object(id, move |obj: PWObject| {
                match obj {
                    PWObject::Link { output_port, active, proxy, .. } => PWObject::Link { input_port: new_input_port, output_port, active, proxy },
                    o => o,
                }
            });
        }
    };

    if let Some(new_output_port) = props.get(&keys::LINK_OUTPUT_PORT) {
        let new_output_port: Id = new_output_port.parse().expect("Failed to parse Link Output Port");
        if new_output_port != output_port {
            info!("Updated Link id: {id} Output Port: {output_port} -> {new_output_port}");
            graph.map_object(id, move |obj: PWObject| {
                match obj {
                    PWObject::Link { input_port, active, proxy, .. } => PWObject::Link { input_port, output_port: new_output_port, active, proxy },
                    o => o,
                }
            });
        }
    };

    if info.change_mask().contains(LinkChangeMask::STATE) {
        let new_active = matches!(info.state(), LinkState::Active);
        if new_active != active {
            info!("Updated Link id: {id} Active: {active} -> {new_active}");
            graph.map_object(id, move |obj: PWObject| {
                match obj {
                    PWObject::Link { input_port, output_port, proxy, .. } => PWObject::Link { input_port, output_port, active: new_active, proxy },
                    o => o,
                }
            });
        }
    }
}

fn registry_global_remove(id: Id, graph: Arc<Mutex<PWGraph>>) {
    info!("Event Registry Global Remove Object id: {id}");
    let mut graph = graph.lock().expect("Failed to lock graph mutex");
    graph.remove(id);
}

fn main() {
    env_logger::init();
    init();

    let graph: Arc<Mutex<PWGraph>> = Arc::new(Mutex::new(PWGraph::new()));
    let idle_state: Arc<Mutex<IdleState>> = Arc::new(Mutex::new(IdleState::new()));

    let mainloop = Rc::new(MainLoop::new().expect("Failed to create mainloop."));
    let context = Rc::new(Context::new(mainloop.as_ref()).expect("Failed to create context."));
    let core = Rc::new(context.connect(None).expect("Failed to get core."));
    let registry = Rc::new(core.get_registry().expect("Failed to get registry"));

    let check_idle_event = {
        let idle_state = Arc::clone(&idle_state);
        let graph = Arc::clone(&graph);
        Rc::new(mainloop.add_event(move || {
            let idle_state = Arc::clone(&idle_state);
            let graph = Arc::clone(&graph);

            let graph = graph.lock().expect("Failed to lock graph mutex");
            let mut idle_state = idle_state.lock().expect("Failed to lock Idle State");
            idle_state.idle = graph.get_active_sinks().len() > 0;
        }))
    };

    let _listener = {
        registry
            .add_listener_local()
            .global({
                let check_idle_event = Rc::clone(&check_idle_event);
                let registry = Rc::clone(&registry);
                let graph = Arc::clone(&graph);
                move |global| {
                    let check_idle_event = Rc::clone(&check_idle_event);
                    let registry = Rc::clone(&registry);
                    let graph = Arc::clone(&graph);
                    match global.type_ {
                        ObjectType::Node => registry_global_node(global, registry, graph, check_idle_event),
                        ObjectType::Port => registry_global_port(global, registry, graph, check_idle_event),
                        ObjectType::Link => registry_global_link(global, registry, graph, check_idle_event),
                        _ => {},
                    }
                }
            })
            .global_remove({
                let graph = Arc::clone(&graph);
                move |id| {
                    let graph = Arc::clone(&graph);
                    registry_global_remove(id, graph)
                }
            })
            .register()
    };

    mainloop.run();
}

