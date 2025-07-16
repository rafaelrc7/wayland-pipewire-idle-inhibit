// Copyright (C) 2023-2025  Rafael Carvalho <contact@rafaelrc.com>

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as published by
// the Free Software Foundation.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

// SPDX-License-Identifier: GPL-3.0-only

//! Connection to the Wayland compositor and manages the Wayland Idle Inhibitor.

use std::io::Write;
use std::os::fd::AsFd;
use std::{error::Error, fmt::Display};

use wayland_client::{
    delegate_noop,
    globals::{registry_queue_init, GlobalList, GlobalListContents},
    protocol::{
        wl_buffer::WlBuffer,
        wl_compositor::WlCompositor,
        wl_registry::{self, WlRegistry},
        wl_shm::{Format, WlShm},
        wl_shm_pool::WlShmPool,
        wl_surface::WlSurface,
    },
    Connection, Dispatch, EventQueue, Proxy, QueueHandle,
};

use wayland_protocols::wp::idle_inhibit::zv1::client::{
    zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1, zwp_idle_inhibitor_v1::ZwpIdleInhibitorV1,
};

use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{self, ZwlrLayerSurfaceV1},
};

use log::{debug, info, trace, warn};

use tempfile;

use super::IdleInhibitor;

/// Wrapper to the Wayland objects and event queue
pub struct WaylandIdleInhibitor {
    event_queue: EventQueue<State>,
    state: State,
}

/// Wayland globals and surface
struct State {
    compositor: WlCompositor,
    shm: WlShm,
    wlr_layer_shell: ZwlrLayerShellV1,
    idle_inhibit_manager: ZwpIdleInhibitManagerV1,
    surface: Option<Surface>,
}

/// Relevant surface objects that depend on each other, thus are represented in a single struct
struct Surface {
    wl_surface: WlSurface,
    wlr_layer_surface: ZwlrLayerSurfaceV1,
    idle_inhibitor: Option<ZwpIdleInhibitorV1>,
    configured: bool,
}

impl WaylandIdleInhibitor {
    /// Creates an instance, including a surface, and fires and treats initial events
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let connection = Connection::connect_to_env()?;
        let (global_list, event_queue) = registry_queue_init::<State>(&connection)?;
        let qhandle = &event_queue.handle();

        let mut state = State::new(&global_list, qhandle)?;
        state.init_new_surface(qhandle);

        let mut wayland_idle_inhibitor = Self { event_queue, state };

        wayland_idle_inhibitor.roundtrip()?;

        let Some(surface) = &wayland_idle_inhibitor.state.surface else {
            warn!(target: "WaylandIdleInhibitor::new", "Missing surface");
            return Err(WaylandIdleInhibitorError::WlSurfaceNotCreated.into());
        };

        surface.create_and_attach_buffer(&wayland_idle_inhibitor.state, qhandle)?;

        debug!(target: "WaylandIdleInhibitor::new", "Instance built");
        Ok(wayland_idle_inhibitor)
    }

    /// Fires enqueued Wayland events to be treated
    fn roundtrip(&mut self) -> Result<usize, wayland_client::DispatchError> {
        self.event_queue.roundtrip(&mut self.state)
    }

    /// Enables or disables Idle inhibiting using the Wayland protocol
    pub fn set_inhibit_idle(&mut self, inhibit_idle: bool) -> Result<(), Box<dyn Error>> {
        let state = &mut self.state;
        let qhandle = &self.event_queue.handle();
        let Some(surface) = &mut state.surface else {
            warn!(target: "WaylandIdleInhibitor::set_inhibit_idle", "Tried to change idle inhibitor status without loaded WlSurface!");
            return Ok(());
        };

        if inhibit_idle {
            if surface.idle_inhibitor.is_none() {
                surface.idle_inhibitor = Some(state.idle_inhibit_manager.create_inhibitor(
                    &surface.wl_surface,
                    qhandle,
                    (),
                ));
                self.roundtrip()?;
                info!(target: "WaylandIdleInhibitor::set_inhibit_idle", "Idle Inhibitor was ENABLED");
            }
        } else if let Some(idle_inhibitor) = &surface.idle_inhibitor {
            idle_inhibitor.destroy();
            surface.idle_inhibitor = None;
            self.roundtrip()?;
            info!(target: "WaylandIdleInhibitor::set_inhibit_idle", "Idle Inhibitor was DISABLED");
        }

        Ok(())
    }
}

impl IdleInhibitor for WaylandIdleInhibitor {
    fn inhibit(&mut self) -> Result<(), Box<dyn Error>> {
        self.set_inhibit_idle(true)
    }

    fn uninhibit(&mut self) -> Result<(), Box<dyn Error>> {
        self.set_inhibit_idle(false)
    }
}

impl State {
    /// Creates an instance by going through the globals list and binding the relevant ones. Does
    /// not create a surface.
    fn new(global_list: &GlobalList, qhandle: &QueueHandle<State>) -> Result<Self, Box<dyn Error>> {
        let compositor: WlCompositor = global_list.bind(qhandle, 6..=6, ())?;
        let shm: WlShm = global_list.bind(qhandle, 1..=1, ())?;
        let wlr_layer_shell: ZwlrLayerShellV1 = global_list.bind(qhandle, 1..=1, ())?;
        let idle_inhibit_manager: ZwpIdleInhibitManagerV1 = global_list.bind(qhandle, 1..=1, ())?;

        Ok(Self {
            compositor,
            shm,
            wlr_layer_shell,
            idle_inhibit_manager,
            surface: None,
        })
    }

    /// Create a new surface, destroying the current one if it exists.
    fn init_new_surface(&mut self, qhandle: &QueueHandle<State>) {
        self.surface = Some(Surface::new(self, qhandle));
    }
}

impl Surface {
    /// Creates an instance. It must receive a [zwlr_layer_surface_v1::Event::Configure] event
    /// before the buffer is created and attached
    fn new(state: &State, qhandle: &QueueHandle<State>) -> Self {
        let wl_surface = state.compositor.create_surface(qhandle, ());
        let wlr_layer_surface = state.wlr_layer_shell.get_layer_surface(
            &wl_surface,
            None,
            Layer::Background,
            "wayland-pipewire-idle-inhibit".into(),
            qhandle,
            (),
        );
        wlr_layer_surface.set_anchor(zwlr_layer_surface_v1::Anchor::all());
        wl_surface.commit();

        Self {
            wl_surface,
            wlr_layer_surface,
            idle_inhibitor: None,
            configured: false,
        }
    }

    /// Creates and attaches a buffer for the surface
    fn create_and_attach_buffer(
        &self,
        state: &State,
        qhandle: &QueueHandle<State>,
    ) -> Result<(), Box<dyn Error>> {
        let width = 1;
        let height = 1;
        let stride = width * 4;
        let pool_size = height * stride * 2;

        if !self.configured {
            warn!(target: "WaylandIdleInhibitor::create_and_attach_buffer",
                "WLR Layer Surface did not receive the configure event!");
            return Err(WaylandIdleInhibitorError::WlrLayerSurfaceNotConfigured.into());
        }

        let mut file = tempfile::tempfile()?;
        file.write_all(b"\x00\x00\x00\x00\x00\x00\x00\x00")?;

        let pool = state.shm.create_pool(file.as_fd(), pool_size, qhandle, ());
        let buffer = pool.create_buffer(0, width, height, stride, Format::Argb8888, qhandle, ());

        self.wl_surface.attach(Some(&buffer), 0, 0);
        self.wl_surface.commit();

        Ok(())
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        if let Some(idle_inhibitor) = &self.idle_inhibitor {
            idle_inhibitor.destroy();
            self.idle_inhibitor = None;
        }
        self.wlr_layer_surface.destroy();
        self.wl_surface.destroy();
    }
}

/// Subscribes to the [ZwlrLayerSurfaceV1] events waiting for configure and closed events.
impl Dispatch<ZwlrLayerSurfaceV1, ()> for State {
    fn event(
        state: &mut Self,
        proxy: &ZwlrLayerSurfaceV1,
        event: <ZwlrLayerSurfaceV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure { serial, .. } => {
                trace!(target: "WaylandIdleInhibitor::ZwlrLayerSurfaceV1::Event::Configure", "WLR Layer Surface Configure Event received");
                if let Some(surface) = &mut state.surface {
                    proxy.ack_configure(serial);
                    surface.configured = true;
                    debug!(target: "WaylandIdleInhibitor::ZwlrLayerSurfaceV1::Event::Configure", "WLR Layer Surface configured");
                } else {
                    warn!(target: "WaylandIdleInhibitor::ZwlrLayerSurfaceV1::Event::Configure",
                        "WLR Layer Surface Configure Event received but Surface is missing!");
                }
            }

            zwlr_layer_surface_v1::Event::Closed => {
                warn!(target: "WaylandIdleInhibitor::ZwlrLayerSurfaceV1::Event::Closed", "WLR Layer Surface was closed");
                state.surface = None;
            }

            _ => unreachable!(),
        }
    }
}

/// Subscribes to the [WlRegistry] events, mainly to treat added and removed objects
impl Dispatch<WlRegistry, GlobalListContents> for State {
    fn event(
        _state: &mut Self,
        _proxy: &WlRegistry,
        event: <WlRegistry as Proxy>::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => {
                trace!(target: "WaylandIdleInhibitor::WlRegistry::Event::Global", "New {} [{}] v{}", interface, name, version);
            }
            wl_registry::Event::GlobalRemove { name } => {
                trace!(target: "WaylandIdleInhibitor::WlRegistry::Event::Global", "Removed {}", name);
            }
            _ => unreachable!(),
        }
    }
}

// These interfaces have no events.
delegate_noop!(State: WlCompositor);
delegate_noop!(State: ZwpIdleInhibitManagerV1);
delegate_noop!(State: ZwpIdleInhibitorV1);
delegate_noop!(State: WlShmPool);
delegate_noop!(State: ZwlrLayerShellV1);

// Ignore events from these object types.
delegate_noop!(State: ignore WlSurface);
delegate_noop!(State: ignore WlBuffer);
delegate_noop!(State: ignore WlShm);

#[derive(Debug, Clone, PartialEq, Eq)]
enum WaylandIdleInhibitorError {
    WlrLayerSurfaceNotConfigured,
    WlSurfaceNotCreated,
}

impl Display for WaylandIdleInhibitorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WaylandIdleInhibitorError::WlrLayerSurfaceNotConfigured => {
                "WLR Layer Surface was not configured!".to_string().fmt(f)
            }
            WaylandIdleInhibitorError::WlSurfaceNotCreated => {
                "Wayland Surface was not created!".to_string().fmt(f)
            }
        }
    }
}

impl Error for WaylandIdleInhibitorError {}
