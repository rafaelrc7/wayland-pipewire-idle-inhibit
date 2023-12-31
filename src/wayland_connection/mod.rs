use wayland_client::{
    protocol::{
        wl_compositor::WlCompositor,
        wl_display::WlDisplay,
        wl_registry::{self, WlRegistry},
        wl_surface::WlSurface,
    },
    Connection, Dispatch, EventQueue, Proxy, QueueHandle,
};
use wayland_protocols::wp::idle_inhibit::zv1::client::{
    zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1, zwp_idle_inhibitor_v1::ZwpIdleInhibitorV1,
};

use log::{info, warn};

pub struct WaylandConnection {
    _connection: Connection,
    _display: WlDisplay,
    event_queue: EventQueue<AppData>,
    qhandle: QueueHandle<AppData>,
    _registry: WlRegistry,
    data: AppData,
}

impl WaylandConnection {
    pub fn new() -> Self {
        let connection = Connection::connect_to_env().unwrap();
        let display = connection.display();
        let event_queue = connection.new_event_queue();
        let qhandle = event_queue.handle();
        let registry = display.get_registry(&qhandle, ());

        let mut obj = Self {
            _connection: connection,
            _display: display,
            event_queue,
            qhandle,
            _registry: registry,
            data: AppData::default(),
        };
        obj.roundtrip();
        obj
    }

    pub fn roundtrip(&mut self) {
        self.event_queue.roundtrip(&mut self.data).unwrap();
    }

    pub fn set_inhibit_idle(&mut self, inhibit_idle: bool) {
        let data = &self.data;
        let Some((idle_manager, _)) = &data.idle_manager else {
            warn!(target: "WaylandConnection::set_inhibit_idle", "Tried to change idle inhibitor status without loaded idle inhibitor manager!");
            return;
        };

        if inhibit_idle {
            if data._idle_inhibitor.is_none() {
                let Some(surface) = &data.surface else {
                    warn!(target: "WaylandConnection::set_inhibit_idle", "Tried to change idle inhibitor status without loaded WlSurface!");
                    return;
                };
                self.data._idle_inhibitor =
                    Some(idle_manager.create_inhibitor(&surface, &self.qhandle, ()));
                self.roundtrip();
                info!(target: "WaylandConnection::set_inhibit_idle", "Idle Inhibitor was ENABLED");
            }
        } else {
            if let Some(indle_inhibitor) = &self.data._idle_inhibitor {
                indle_inhibitor.destroy();
                self.data._idle_inhibitor = None;
                self.roundtrip();
                info!(target: "WaylandConnection::set_inhibit_idle", "Idle Inhibitor was DISABLED");
            }
        }
    }
}

#[derive(Default)]
struct AppData {
    compositor: Option<(WlCompositor, u32)>,
    surface: Option<WlSurface>,
    idle_manager: Option<(ZwpIdleInhibitManagerV1, u32)>,
    _idle_inhibitor: Option<ZwpIdleInhibitorV1>,
}

impl Dispatch<WlRegistry, ()> for AppData {
    fn event(
        state: &mut Self,
        registry: &WlRegistry,
        event: <WlRegistry as wayland_client::Proxy>::Event,
        data: &(),
        _conn: &wayland_client::Connection,
        qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => {
                if interface == WlCompositor::interface().name && state.compositor.is_none() {
                    info!(target: "WaylandConnection::WlRegistry::Event::Global", "Adding Compositor with name {name} and version {version}");
                    let compositor: WlCompositor = registry.bind(name, version, qhandle, *data);
                    state.surface = Some(compositor.create_surface(qhandle, *data));
                    state.compositor = Some((compositor, name));
                } else if interface == ZwpIdleInhibitManagerV1::interface().name
                    && state.idle_manager.is_none()
                {
                    info!(target: "WaylandConnection::WlRegistry::Event::Global", "Adding IdleInhibitManager with name {name} and version {version}");
                    state.idle_manager = Some((registry.bind(name, version, qhandle, *data), name));
                };
            }
            wl_registry::Event::GlobalRemove { name } => {
                if let Some((_, compositor_name)) = &state.compositor {
                    if name == *compositor_name {
                        warn!(target: "WaylandConnection::GlobalRemove", "Compositor was removed!");
                        state.compositor = None;
                        state.surface = None;
                    }
                } else if let Some((_, idle_manager_name)) = &state.idle_manager {
                    if name == *idle_manager_name {
                        warn!(target: "WaylandConnection::GlobalRemove", "IdleInhibitManager was removed!");
                        state.idle_manager = None;
                    }
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<WlCompositor, ()> for AppData {
    fn event(
        _state: &mut Self,
        _registry: &WlCompositor,
        _event: <WlCompositor as Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
    } // This interface has no events.
}

impl Dispatch<WlSurface, ()> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &WlSurface,
        _event: <WlSurface as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpIdleInhibitManagerV1, ()> for AppData {
    fn event(
        _state: &mut Self,
        _registry: &ZwpIdleInhibitManagerV1,
        _event: <ZwpIdleInhibitManagerV1 as Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
    } // This interface has no events.
}

impl Dispatch<ZwpIdleInhibitorV1, ()> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &ZwpIdleInhibitorV1,
        _event: <ZwpIdleInhibitorV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    } // This interface has no events.
}
