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

use std::collections::HashMap;
use std::error::Error;
use std::iter::repeat_with;
use std::os::fd::{AsFd, OwnedFd};

use nix::errno::Errno;
use nix::fcntl::OFlag;
use nix::sys::mman::{shm_open, shm_unlink};
use nix::sys::stat::Mode;
use nix::unistd::ftruncate;
use wayland_client::backend::ObjectId;
use wayland_client::protocol::wl_buffer;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::{
    delegate_noop,
    globals::{registry_queue_init, GlobalListContents},
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
    zwlr_layer_shell_v1::{self, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{self, ZwlrLayerSurfaceV1},
};

use super::IdleInhibitor;

// Structs

pub type WaylandEventQueue = EventQueue<WaylandIdleInhibitor>;

/// Wayland Idle Inhibitor
#[derive(Debug)]
pub struct WaylandIdleInhibitor {
    compositor: WlCompositor,
    qhandle: QueueHandle<Self>,
    shm: WlShm,
    wlr_layer_shell: ZwlrLayerShellV1,
    idle_inhibit_manager: ZwpIdleInhibitManagerV1,
    outputs: HashMap<u32, Output>, // The u32 key represents a proxy name, the ID used by Wayland

    is_idle_inhibited: bool,
}

/// Wayland [WlOutput] and it's [Surface]
#[derive(Debug)]
struct Output {
    wl_output: WlOutput,
    surface: Option<Surface>,
}

/// Relevant surface objects that depend on each other, thus are represented in a single struct
#[derive(Debug)]
struct Surface {
    wl_surface: WlSurface,
    wlr_layer_surface: ZwlrLayerSurfaceV1,
    idle_inhibitor: Option<SurfaceIdleInhibitor>,
}

/// Wrapper around the [ZwpIdleInhibitorV1] type for the implemenation of the [Drop] trait
#[derive(Debug)]
struct SurfaceIdleInhibitor(ZwpIdleInhibitorV1);

// Struct implemenations

impl WaylandIdleInhibitor {
    /// Creates an instance by going through the globals list and binding the relevant ones. Does
    /// not create a surface.
    pub fn new() -> Result<(Self, WaylandEventQueue), Box<dyn Error>> {
        let connection = Connection::connect_to_env()?;
        let (global_list, mut event_queue) = registry_queue_init::<Self>(&connection)?;
        let qhandle = event_queue.handle();

        let compositor: WlCompositor = global_list.bind(&qhandle, 1..=1, ())?;
        let shm: WlShm = global_list.bind(&qhandle, 1..=1, ())?;
        let wlr_layer_shell: ZwlrLayerShellV1 = global_list.bind(&qhandle, 1..=1, ())?;
        let idle_inhibit_manager: ZwpIdleInhibitManagerV1 =
            global_list.bind(&qhandle, 1..=1, ())?;

        let registry: &WlRegistry = global_list.registry();

        let outputs: HashMap<u32, Output> = global_list
            .contents()
            .clone_list()
            .iter()
            .filter_map(|global| {
                if global.interface == WlOutput::interface().name {
                    Some((
                        global.name,
                        Output::new(registry.bind(global.name, 1, &qhandle, ())),
                    ))
                } else {
                    None
                }
            })
            .collect();

        let mut obj = Self {
            compositor,
            qhandle,
            shm,
            wlr_layer_shell,
            idle_inhibit_manager,
            outputs,
            is_idle_inhibited: false,
        };
        obj.init_missing_surfaces();

        event_queue.roundtrip(&mut obj)?;

        Ok((obj, event_queue))
    }

    /// Create surfaces for all outputs that do not already have one
    fn init_missing_surfaces(&mut self) {
        log::debug!(target: "WaylandIdleInhibitor::init_surfaces", "Initialising missing surfaces");
        let missing_surface_outputs: Vec<u32> = self
            .outputs
            .iter()
            .filter_map(|(k, v)| if v.surface.is_none() { Some(*k) } else { None })
            .collect();

        if missing_surface_outputs.is_empty() {
            log::debug!(target: "WaylandIdleInhibitor::init_surfaces", "No new surfaces need to be created");
            return;
        }

        for output_id in missing_surface_outputs {
            let Some(output) = self.outputs.get(&output_id) else {
                continue;
            };

            let mut surface = Surface::new(self, &self.qhandle, &output.wl_output);
            surface.set_inhibit_idle(
                self.is_idle_inhibited,
                &self.idle_inhibit_manager,
                &self.qhandle,
            );

            let Some(output) = self.outputs.get_mut(&output_id) else {
                continue;
            };
            output.surface = Some(surface);
            log::debug!(target: "WaylandIdleInhibitor::init_surfaces", "Created surface for {}", output.wl_output.id());
        }
    }

    /// Find an output proxy name (u32) from a related wlr_layer_surface id
    fn find_wlr_layer_surface_output(&self, id: &ObjectId) -> Option<&u32> {
        self.outputs.iter().find_map(|(k, v)| {
            if v.surface
                .as_ref()
                .is_some_and(|s| s.wlr_layer_surface.id() == *id)
            {
                Some(k)
            } else {
                None
            }
        })
    }

    /// Enables or disables Idle inhibiting using the Wayland protocol, using a
    /// [ZwpIdleInhibitorV1] for each [Surface]
    pub fn set_inhibit_idle(&mut self, inhibit_idle: bool) -> Result<(), Box<dyn Error>> {
        self.is_idle_inhibited = inhibit_idle;

        let surfaces: Vec<&mut Surface> = self
            .outputs
            .iter_mut()
            .filter_map(|(_, v)| v.surface.as_mut())
            .collect();

        if surfaces.is_empty() {
            log::debug!(target: "WaylandIdleInhibitor::set_inhibit_idle", "No surfaces loaded");
            return Ok(());
        }

        let mut changed_value = false;
        for surface in surfaces {
            changed_value =
                surface.set_inhibit_idle(inhibit_idle, &self.idle_inhibit_manager, &self.qhandle)
                    || changed_value;
        }

        if changed_value {
            //self.roundtrip()?;
            log::info!(target: "WaylandIdleInhibitor::set_inhibit_idle", "Idle Inhibitor was {}", if inhibit_idle {"ENABLED"} else {"DISABLED"});
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

impl Output {
    fn new(wl_output: WlOutput) -> Self {
        Self {
            wl_output,
            surface: None,
        }
    }
}

impl Surface {
    /// Creates an instance. It must receive a [zwlr_layer_surface_v1::Event::Configure] event
    /// before the buffer is created and attached
    fn new(
        state: &WaylandIdleInhibitor,
        qhandle: &QueueHandle<WaylandIdleInhibitor>,
        output: &WlOutput,
    ) -> Self {
        let wl_surface = state.compositor.create_surface(qhandle, ());
        let wlr_layer_surface = state.wlr_layer_shell.get_layer_surface(
            &wl_surface,
            Some(output),
            zwlr_layer_shell_v1::Layer::Background,
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
        }
    }

    /// Creates and attaches a buffer for the surface. Must be called after the
    /// [zwlr_layer_surface_v1::Event::Configure] event.
    fn configure(
        &self,
        state: &WaylandIdleInhibitor,
        qhandle: &QueueHandle<WaylandIdleInhibitor>,
    ) -> Result<(), Box<dyn Error>> {
        let width: i32 = 1;
        let height: i32 = 1;
        let stride: i32 = width * 4;
        let pool_size: i32 = height * stride * 2;

        let shm = Self::allocate_shm_file(pool_size as i64)?;

        let pool = state.shm.create_pool(shm.as_fd(), pool_size, qhandle, ());
        let buffer = pool.create_buffer(0, width, height, stride, Format::Argb8888, qhandle, ());

        self.wl_surface.attach(Some(&buffer), 0, 0);
        self.wl_surface.commit();

        pool.destroy(); // Destroys Pool when all buffers are gone

        Ok(())
    }

    /// Creates a shm file, unlinks it (so that it gets removed when closed) and allocates the
    /// requested number of bytes.
    fn allocate_shm_file(size: i64) -> Result<OwnedFd, Box<dyn Error>> {
        let (shm, shm_name) = Self::create_shm_file()?;

        shm_unlink(shm_name.as_str())?;
        ftruncate(&shm, size)?;

        Ok(shm)
    }

    /// Creates a shm file with a random name. In case of name conflicts it retries the process
    /// multiple times.
    fn create_shm_file() -> Result<(OwnedFd, String), Box<dyn Error>> {
        let mut rng = fastrand::Rng::new();
        let mut retries: u32 = 100;

        loop {
            if retries == 0 {
                break Err(Box::new(Errno::EEXIST));
            }
            retries = retries.saturating_sub(1);

            let shm_name_suffix: String = repeat_with(|| rng.alphanumeric()).take(10).collect();
            let shm_name = format!("/wayland-pipewire-idle-inhibit-buffer-{}", shm_name_suffix);

            let shm = shm_open(
                shm_name.as_str(),
                OFlag::O_CREAT | OFlag::O_EXCL | OFlag::O_RDWR,
                Mode::S_IWUSR | Mode::S_IRUSR,
            );

            match shm {
                Ok(shm) => break Ok((shm, shm_name)),
                Err(Errno::EEXIST) => continue,
                Err(err) => break Err(Box::new(err)),
            }
        }
    }

    /// Create or destroy the surface's [ZwpIdleInhibitorV1]. Returns true if state was changed,
    /// false otherwise.
    fn set_inhibit_idle(
        &mut self,
        inhibit_idle: bool,
        idle_inhibit_manager: &ZwpIdleInhibitManagerV1,
        qhandle: &QueueHandle<WaylandIdleInhibitor>,
    ) -> bool {
        if inhibit_idle {
            if self.idle_inhibitor.is_none() {
                self.idle_inhibitor = Some(SurfaceIdleInhibitor(
                    idle_inhibit_manager.create_inhibitor(&self.wl_surface, qhandle, ()),
                ));
                log::debug!(target: "WaylandIdleInhibitor::Surface::set_inhibit_idle", "Idle Inhibitor was ENABLED for {}", self.wl_surface.id());
                return true;
            }
        } else if self.idle_inhibitor.is_some() {
            self.idle_inhibitor = None;
            log::debug!(target: "WaylandIdleInhibitor::Surface::set_inhibit_idle", "Idle Inhibitor was DISABLED for {}", self.wl_surface.id());
            return true;
        }
        false
    }
}

// Drop implementations to release Wayland resources. Objects are kepf even after the proxy goes
// out of scope. Thus, we need to manually call the `destroy` destructor.

impl Drop for WaylandIdleInhibitor {
    fn drop(&mut self) {
        self.idle_inhibit_manager.destroy();
        self.shm.release();
        self.wlr_layer_shell.destroy();
    }
}

impl Drop for Output {
    fn drop(&mut self) {
        self.wl_output.release();
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        self.wlr_layer_surface.destroy();
        self.wl_surface.destroy();
    }
}

impl Drop for SurfaceIdleInhibitor {
    fn drop(&mut self) {
        let SurfaceIdleInhibitor(idle_inhibitor) = self;
        idle_inhibitor.destroy();
    }
}

// Event callback implementations

/// Subscribes to the [ZwlrLayerSurfaceV1] events waiting for configure and closed events.
impl Dispatch<ZwlrLayerSurfaceV1, ()> for WaylandIdleInhibitor {
    fn event(
        state: &mut Self,
        proxy: &ZwlrLayerSurfaceV1,
        event: <ZwlrLayerSurfaceV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure { serial, .. } => {
                log::trace!(target: "WaylandIdleInhibitor::ZwlrLayerSurfaceV1::Event::Configure", "Event received");
                let Some(output_id) = state.find_wlr_layer_surface_output(&proxy.id()) else {
                    log::debug!(target: "WaylandIdleInhibitor::ZwlrLayerSurfaceV1::Event::Configure", "Output not found");
                    return;
                };
                if let Some(surface) = &state
                    .outputs
                    .get(output_id)
                    .and_then(|o| o.surface.as_ref())
                {
                    surface.wlr_layer_surface.ack_configure(serial);
                    if let Err(error) = surface.configure(state, qhandle) {
                        log::error!(target: "WaylandIdleInhibitor::ZwlrLayerSurfaceV1::Event::Configure", "{}", error);
                        return;
                    }
                    log::debug!(target: "WaylandIdleInhibitor::ZwlrLayerSurfaceV1::Event::Configure", "Configured");
                };
            }

            zwlr_layer_surface_v1::Event::Closed => {
                log::debug!(target: "WaylandIdleInhibitor::ZwlrLayerSurfaceV1::Event::Closed", "Surface {}", proxy.id());
                let Some(output_id) = state.find_wlr_layer_surface_output(&proxy.id()) else {
                    return;
                };
                let Some(output) = &mut state.outputs.get_mut(&output_id.to_owned()) else {
                    return;
                };
                output.surface = None;
            }

            _ => {}
        }
    }
}

/// Subscribes to the [WlRegistry] events, mainly to treat added and removed objects
impl Dispatch<WlRegistry, GlobalListContents> for WaylandIdleInhibitor {
    fn event(
        state: &mut Self,
        proxy: &WlRegistry,
        event: <WlRegistry as Proxy>::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name, interface, ..
            } => {
                log::trace!(target: "WaylandIdleInhibitor::WlRegistry::Event::Global", "New {} [{}] v{}", interface, name, 1);
                if interface == WlOutput::interface().name {
                    log::debug!(target: "WaylandIdleInhibitor::WlRegistry::Event::Global", "New output {}", name);
                    let wl_output = proxy.bind(name, 1, qhandle, ());
                    state.outputs.insert(name, Output::new(wl_output));
                    state.init_missing_surfaces();
                }
            }
            wl_registry::Event::GlobalRemove { name } => {
                log::trace!(target: "WaylandIdleInhibitor::WlRegistry::Event::Global", "Removed {}", name);
                if state.outputs.remove(&name).is_some() {
                    log::debug!(target: "WaylandIdleInhibitor::WlRegistry::Event::GlobalRemove", "Removed output {}", name);
                }
            }
            _ => {}
        }
    }
}

/// Subscribes to the [WlBuffer] events, to destroy the buffer when it is time.
impl Dispatch<WlBuffer, ()> for WaylandIdleInhibitor {
    fn event(
        _state: &mut Self,
        proxy: &WlBuffer,
        event: <WlBuffer as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if let wl_buffer::Event::Release = event {
            proxy.destroy();
        }
    }
}

// Ignore events from these object types.
delegate_noop!(WaylandIdleInhibitor: ignore WlOutput);
delegate_noop!(WaylandIdleInhibitor: ignore WlShm);
delegate_noop!(WaylandIdleInhibitor: ignore WlSurface);

delegate_noop!(WaylandIdleInhibitor: ignore WlCompositor);
delegate_noop!(WaylandIdleInhibitor: ignore WlShmPool);
delegate_noop!(WaylandIdleInhibitor: ignore ZwlrLayerShellV1);
delegate_noop!(WaylandIdleInhibitor: ignore ZwpIdleInhibitManagerV1);
delegate_noop!(WaylandIdleInhibitor: ignore ZwpIdleInhibitorV1);
