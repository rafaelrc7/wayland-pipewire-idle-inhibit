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

//! Connection to the Wayland compositor and manages the Wayland Idle Inhibitor.

use std::error::Error;
use std::os::fd::AsFd;
use std::io::Write;

use wayland_client::{
    protocol::{
        wl_buffer::WlBuffer,
        wl_compositor::WlCompositor,
        wl_display::WlDisplay,
        wl_registry::{self, WlRegistry},
        wl_surface::WlSurface,
        wl_shm::{WlShm, Format},
        wl_shm_pool::WlShmPool,
    },
    Connection, Dispatch, DispatchError, EventQueue, Proxy, QueueHandle,
};

use wayland_protocols::wp::idle_inhibit::zv1::client::{
    zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1, zwp_idle_inhibitor_v1::ZwpIdleInhibitorV1,
};

use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{self, ZwlrLayerSurfaceV1},
};

use log::{debug, info, warn};

use tempfile;

use super::IdleInhibitor;

/// Wrapper to the Wayland objects and the idle inhibitor protocol
pub struct WaylandIdleInhibitor {
    _connection: Connection,
    _display: WlDisplay,
    event_queue: EventQueue<AppData>,
    qhandle: QueueHandle<AppData>,
    _registry: WlRegistry,
    data: AppData,
}

impl IdleInhibitor for WaylandIdleInhibitor {
    fn inhibit(&mut self) -> Result<(), Box<dyn Error>> {
        self.set_inhibit_idle(true)
    }

    fn uninhibit(&mut self) -> Result<(), Box<dyn Error>> {
        self.set_inhibit_idle(false)
    }
}

impl WaylandIdleInhibitor {
    /// Builds the connection struct and fires the initial events, necessary to receive and store
    /// wayland objects
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let connection = Connection::connect_to_env()?;
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
        obj.initialize()?;
        Ok(obj)
    }

    fn init_buffer(&mut self) {
        let mut file = tempfile::tempfile().unwrap();
        let shm = self.data.shm.as_ref().expect("WlShm global not initialized");
        let width = 1;
        let height = 1;
        let stride = width * 4;
        let pool_size = height * stride * 2;
        let pool = shm.create_pool(file.as_fd(), pool_size, &self.qhandle, ());
        let buffer = pool.create_buffer(
            0,
            width,
            height,
            stride,
            Format::Argb8888,
            &self.qhandle,
            (),
        );
        let _ = file.write(b"\x00\x00\x00\x00\x00\x00\x00\x00");
        self.data.buffer = Some(buffer);
    }

    fn init_layer_surface(&mut self) {
        let layer_shell = self.data.layer_shell.as_ref().expect("ZwlrLayerShellV1 global not initialized");
        let surface = self.data.surface.as_ref().expect("WlSurface not created");
        let layer_surface = layer_shell.get_layer_surface(
            surface,
            None,
            Layer::Background,
            "wayland-pipewire-idle-inhibit".to_string(),
            &self.qhandle,
            (),
        );
        layer_surface.set_anchor(zwlr_layer_surface_v1::Anchor::all());
        surface.commit();
        self.data.layer_surface = Some(layer_surface);
    }

    fn initialize(&mut self) -> Result<(), Box<dyn Error>> {
        self.roundtrip()?; // init globals

        self.init_buffer();
        self.init_layer_surface();
        self.roundtrip()?; // make sure layer_surface receives the configure event
        let surface = self.data.surface.as_ref().expect("WlSurface is not initialized");
        let buffer = self.data.buffer.as_ref().expect("WlBuffer is not initialized");
        
        surface.attach(Some(buffer), 0, 0);
        surface.commit();
        self.roundtrip()?;

        Ok(())
    }

    /// Fires enqueued Wayland events to be treated
    pub fn roundtrip(&mut self) -> Result<usize, DispatchError> {
        self.event_queue.roundtrip(&mut self.data)
    }

    /// Enables or disables Idle inhibiting using the Wayland protocol
    pub fn set_inhibit_idle(&mut self, inhibit_idle: bool) -> Result<(), Box<dyn Error>> {
        let data = &self.data;
        let Some((idle_manager, _)) = &data.idle_manager else {
            warn!(target: "WaylandIdleInhibitor::set_inhibit_idle", "Tried to change idle inhibitor status without loaded idle inhibitor manager!");
            return Ok(());
        };

        if inhibit_idle {
            if data._idle_inhibitor.is_none() {
                let Some(surface) = &data.surface else {
                    warn!(target: "WaylandIdleInhibitor::set_inhibit_idle", "Tried to change idle inhibitor status without loaded WlSurface!");
                    return Ok(());
                };
                self.data._idle_inhibitor =
                    Some(idle_manager.create_inhibitor(surface, &self.qhandle, ()));
                self.roundtrip()?;
                info!(target: "WaylandIdleInhibitor::set_inhibit_idle", "Idle Inhibitor was ENABLED");
            }
        } else if let Some(indle_inhibitor) = &self.data._idle_inhibitor {
            indle_inhibitor.destroy();
            self.data._idle_inhibitor = None;
            self.roundtrip()?;
            info!(target: "WaylandIdleInhibitor::set_inhibit_idle", "Idle Inhibitor was DISABLED");
        }

        Ok(())
    }
}

/// Wayland connection and main objects
#[derive(Default)]
struct AppData {
    compositor: Option<(WlCompositor, u32)>,
    surface: Option<WlSurface>,
    shm: Option<WlShm>,
    buffer: Option<WlBuffer>,
    layer_shell: Option<ZwlrLayerShellV1>,
    layer_surface: Option<ZwlrLayerSurfaceV1>,
    idle_manager: Option<(ZwpIdleInhibitManagerV1, u32)>,
    _idle_inhibitor: Option<ZwpIdleInhibitorV1>,
}

/// Subscribes to the [WlRegistry] events, mainly to treat added and removed objects
impl Dispatch<WlRegistry, ()> for AppData {
    fn event(
        state: &mut Self,
        proxy: &WlRegistry,
        event: <WlRegistry as wayland_client::Proxy>::Event,
        _data: &(),
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
                    debug!(target: "WaylandIdleInhibitor::WlRegistry::Event::Global", "Adding Compositor with name {name} and version {version}");
                    let compositor: WlCompositor = proxy.bind(name, version, qhandle, ());
                    state.surface = Some(compositor.create_surface(qhandle, ()));
                    state.compositor = Some((compositor, name));
                } else if interface == WlShm::interface().name {
                    state.shm = Some(proxy.bind(name, version, qhandle, ()));
                } else if interface == ZwlrLayerShellV1::interface().name
                    && state.layer_shell.is_none()
                {
                    state.layer_shell = Some(proxy.bind(name, version, qhandle, ()));
                } else if interface == ZwpIdleInhibitManagerV1::interface().name
                    && state.idle_manager.is_none()
                {
                    debug!(target: "WaylandIdleInhibitor::WlRegistry::Event::Global", "Adding IdleInhibitManager with name {name} and version {version}");
                    state.idle_manager = Some((proxy.bind(name, version, qhandle, ()), name));
                };
            }
            wl_registry::Event::GlobalRemove { name } => {
                if let Some((_, compositor_name)) = &state.compositor {
                    if name == *compositor_name {
                        warn!(target: "WaylandIdleInhibitor::GlobalRemove", "Compositor was removed!");
                        state.compositor = None;
                        state.surface = None;
                    }
                } else if let Some((_, idle_manager_name)) = &state.idle_manager {
                    if name == *idle_manager_name {
                        warn!(target: "WaylandIdleInhibitor::GlobalRemove", "IdleInhibitManager was removed!");
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
        _proxy: &WlCompositor,
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
        // no-op
    }
}

impl Dispatch<ZwpIdleInhibitManagerV1, ()> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &ZwpIdleInhibitManagerV1,
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

impl Dispatch<WlShm, ()> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &WlShm,
        _event: <WlShm as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    } // This interface has no events.
}

impl Dispatch<WlShmPool, ()> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &WlShmPool,
        _event: <WlShmPool as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    } // This interface has no events.
}

impl Dispatch<WlBuffer, ()> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &WlBuffer,
        _event: <WlBuffer as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        // no-op
    }
}

impl Dispatch<ZwlrLayerShellV1, ()> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrLayerShellV1,
        _event: <ZwlrLayerShellV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    } // This interface has no events.
}

impl Dispatch<ZwlrLayerSurfaceV1, ()> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrLayerSurfaceV1,
        _event: <ZwlrLayerSurfaceV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if let zwlr_layer_surface_v1::Event::Configure { serial, .. } = _event {
            _proxy.ack_configure(serial);
        }
    }
}
