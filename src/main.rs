mod pipewire_connection;

use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::{collections::HashMap, rc::Rc};

use log::{debug, error, info, log, trace, warn};
use pipewire::context::Context;
use pipewire::link::{Link, LinkChangeMask, LinkInfo, LinkListener, LinkState};
use pipewire::loop_::EventSource;
use pipewire::main_loop::MainLoop;
use pipewire::node::{NodeInfo, NodeListener};
use pipewire::port::{Port, PortInfo, PortListener};
use pipewire::proxy::{Listener, ProxyT};
use pipewire::registry::{GlobalObject, Registry};
use pipewire::spa::utils::dict::DictRef;
use pipewire::spa::utils::Direction;
use pipewire::{init, keys, node::Node, types::ObjectType};

use pipewire_connection::graph::{
    Id, LinkData, NodeData, PWGraph, PWObject, PWObjectData, PortData, Proxy,
};

#[derive(Debug)]
enum PWSignal {
    CheckIdleEvent,
    Terminate,
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

fn registry_global_node(
    node: &GlobalObject<&DictRef>,
    registry: Rc<Registry>,
    graph: Rc<RefCell<PWGraph>>,
    update_event: mpsc::Sender<()>,
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
            let update_event = update_event.clone();
            move |info| node_info(info, Rc::clone(&graph), update_event.clone())
        })
        .register();

    //info!("Event Registry Global Created Node id: {id}\tname: {name}\tmedia class: {media_class}");
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

    update_event.send(()).unwrap();
}

fn node_info(info: &NodeInfo, graph: Rc<RefCell<PWGraph>>, update_event: mpsc::Sender<()>) {
    let id = info.id();
    info!("Event Node Info id:{id}");

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
        update_event.send(()).unwrap();
    }
}

fn direction_from_string(direction: &str) -> Option<Direction> {
    match direction {
        "out" => Some(Direction::Output),
        "in" => Some(Direction::Input),
        _ => None,
    }
}

fn registry_global_port(
    port: &GlobalObject<&DictRef>,
    registry: Rc<Registry>,
    graph: Rc<RefCell<PWGraph>>,
    update_event: mpsc::Sender<()>,
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
            let update_event = update_event.clone();
            move |info| port_info(info, Rc::clone(&graph), update_event.clone())
        })
        .param(move |_, _param_id, _, _, _param| {}) // TODO
        .register();

    //info!("Event Registry Global Created Port ID: {id}\tNode ID: {node_id}\tName: {name}\tDirection: {:?}\tTerminal: {is_terminal}", direction);
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

    update_event.send(()).unwrap();
}

fn port_info(info: &PortInfo, graph: Rc<RefCell<PWGraph>>, update_event: mpsc::Sender<()>) {
    let id = info.id();
    info!("Event Port Info id:{id}");

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
        update_event.send(()).unwrap();
    }
}

fn registry_global_link(
    link: &GlobalObject<&DictRef>,
    registry: Rc<Registry>,
    graph: Rc<RefCell<PWGraph>>,
    update_event: mpsc::Sender<()>,
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
            let update_event = update_event.clone();
            move |info| link_info(info, Rc::clone(&graph), update_event.clone())
        })
        .register();

    //info!("Event Registry Global Created Link ID: {id}\tInput Port ID: {input_port}\tOutput Port ID: {output_port}");
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

    update_event.send(()).unwrap();
}

fn link_info(info: &LinkInfo, graph: Rc<RefCell<PWGraph>>, update_event: mpsc::Sender<()>) {
    let id = info.id();
    info!("Event Link Info id:{id}");

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
        update_event.send(()).unwrap();
    }
}

fn registry_global_remove(id: Id, graph: Rc<RefCell<PWGraph>>, update_event: mpsc::Sender<()>) {
    info!("Event Registry Global Remove Object id: {id}");
    graph.borrow_mut().remove(id);

    update_event.send(()).unwrap();
}

fn pw_thread(main_sender: mpsc::Sender<()>, pw_receiver: pipewire::channel::Receiver<PWSignal>) {
    let mainloop = MainLoop::new().expect("Failed to create mainloop.");

    let graph = Rc::new(RefCell::new(PWGraph::new()));
    let idle_state: Arc<Mutex<IdleState>> = Arc::new(Mutex::new(IdleState::new()));

    let context = Rc::new(Context::new(&mainloop).expect("Failed to create context."));
    let core = Rc::new(context.connect(None).expect("Failed to get core."));
    let registry = Rc::new(core.get_registry().expect("Failed to get registry"));

    let _listener = {
        registry
            .add_listener_local()
            .global({
                let registry = Rc::clone(&registry);
                let graph = Rc::clone(&graph);
                let main_sender = main_sender.clone();

                move |global| {
                    let registry = Rc::clone(&registry);
                    let graph = Rc::clone(&graph);

                    match global.type_ {
                        ObjectType::Node => {
                            registry_global_node(global, registry, graph, main_sender.clone())
                        }
                        ObjectType::Port => {
                            registry_global_port(global, registry, graph, main_sender.clone())
                        }
                        ObjectType::Link => {
                            registry_global_link(global, registry, graph, main_sender.clone())
                        }
                        _ => {}
                    }
                }
            })
            .global_remove({
                let graph = Rc::clone(&graph);
                let main_sender = main_sender.clone();

                move |id| {
                    registry_global_remove(id, Rc::clone(&graph), main_sender.clone());
                }
            })
            .register()
    };

    let _receiver = pw_receiver.attach(&mainloop, {
        let mainloop = mainloop.clone();

        move |signal: PWSignal| match signal {
            PWSignal::Terminate => {
                mainloop.quit();
            }
            PWSignal::CheckIdleEvent => {
                // _check_idle_event.signal();

                let graph = graph.borrow_mut();
                let mut idle_state = idle_state.lock().expect("Failed to lock Idle State");
                idle_state.idle = !graph.get_active_sinks().is_empty();

                dbg!(idle_state.idle);
            }
        }
    });

    mainloop.run();
}

fn main() {
    env_logger::init();
    let (main_sender, main_receiver) = mpsc::channel();
    let (pw_sender, pw_receiver) = pipewire::channel::channel();

    let _pw_thread = thread::spawn(move || pw_thread(main_sender, pw_receiver));

    // while let () = main_receiver.recv().unwrap() {
    //     pw_sender.send(PWSignal::CheckIdleEvent);
    // }

    loop {
        main_receiver.recv().unwrap();
        pw_sender.send(PWSignal::CheckIdleEvent).unwrap();
    }

    // pw_sender.send(PWSignal::Terminate);
    // pw_thread.join();
}
