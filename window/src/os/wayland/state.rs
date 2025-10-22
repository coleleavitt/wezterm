use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use smithay_client_toolkit::compositor::{CompositorState, SurfaceData};
use smithay_client_toolkit::data_device_manager::data_device::DataDevice;
use smithay_client_toolkit::data_device_manager::data_source::CopyPasteSource;
use smithay_client_toolkit::data_device_manager::DataDeviceManagerState;
use smithay_client_toolkit::globals::GlobalData;
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::primary_selection::device::PrimarySelectionDevice;
use smithay_client_toolkit::primary_selection::selection::PrimarySelectionSource;
use smithay_client_toolkit::primary_selection::PrimarySelectionManagerState;
use smithay_client_toolkit::reexports::protocols_wlr::output_management::v1::client::zwlr_output_head_v1::ZwlrOutputHeadV1;
use smithay_client_toolkit::reexports::protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;
use smithay_client_toolkit::reexports::protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::ZwlrOutputModeV1;
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::seat::pointer::ThemedPointer;
use smithay_client_toolkit::seat::SeatState;
use smithay_client_toolkit::shell::xdg::XdgShell;
use smithay_client_toolkit::shm::slot::SlotPool;
use smithay_client_toolkit::shm::{Shm, ShmHandler};
use smithay_client_toolkit::subcompositor::SubcompositorState;
use smithay_client_toolkit::{
    delegate_compositor, delegate_data_device, delegate_output, delegate_pointer, delegate_primary_selection, delegate_registry, delegate_seat, delegate_shm, delegate_subcompositor, delegate_xdg_shell, delegate_xdg_window, registry_handlers
};
use wayland_client::backend::ObjectId;
use wayland_client::globals::GlobalList;
use wayland_client::protocol::wl_keyboard::WlKeyboard;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{delegate_dispatch, Connection, QueueHandle};
use wayland_protocols::wp::commit_timing::v1::client::wp_commit_timing_manager_v1::WpCommitTimingManagerV1;
use wayland_protocols::wp::commit_timing::v1::client::wp_commit_timer_v1::WpCommitTimerV1;
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1;
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1::WpFractionalScaleV1;
use wayland_protocols::wp::input_timestamps::zv1::client::zwp_input_timestamps_manager_v1::ZwpInputTimestampsManagerV1;
use wayland_protocols::wp::input_timestamps::zv1::client::zwp_input_timestamps_v1::ZwpInputTimestampsV1;
use wayland_protocols::wp::viewporter::client::wp_viewport::WpViewport;
use wayland_protocols::wp::viewporter::client::wp_viewporter::WpViewporter;
use wayland_protocols::wp::linux_drm_syncobj::v1::client::wp_linux_drm_syncobj_manager_v1::WpLinuxDrmSyncobjManagerV1;
use wayland_protocols::wp::linux_drm_syncobj::v1::client::wp_linux_drm_syncobj_surface_v1::WpLinuxDrmSyncobjSurfaceV1;
use wayland_protocols::wp::linux_drm_syncobj::v1::client::wp_linux_drm_syncobj_timeline_v1::WpLinuxDrmSyncobjTimelineV1;
use wayland_protocols::wp::presentation_time::client::wp_presentation::WpPresentation;
use wayland_protocols::wp::presentation_time::client::wp_presentation_feedback::WpPresentationFeedback;
use wayland_protocols::wp::tearing_control::v1::client::wp_tearing_control_manager_v1::WpTearingControlManagerV1;
use wayland_protocols::wp::tearing_control::v1::client::wp_tearing_control_v1::WpTearingControlV1;
use wayland_protocols::wp::text_input::zv3::client::zwp_text_input_manager_v3::ZwpTextInputManagerV3;
use wayland_protocols::wp::text_input::zv3::client::zwp_text_input_v3::ZwpTextInputV3;
use wayland_protocols_plasma::blur::client::org_kde_kwin_blur_manager::OrgKdeKwinBlurManager;

use crate::x11::KeyboardWithFallback;

use super::inputhandler::{TextInputData, TextInputState};
use super::pointer::{PendingMouse, PointerUserData};
use super::{OutputManagerData, OutputManagerState, SurfaceUserData, WaylandWindowInner};

// We can't combine WaylandState and WaylandConnection together because
// the run_message_loop has &self(WaylandConnection) and needs to update WaylandState as mut
pub(super) struct WaylandState {
    registry: RegistryState,
    pub(super) output: OutputState,
    pub(super) compositor: CompositorState,
    pub(super) subcompositor: Arc<SubcompositorState>,
    pub(super) text_input: Option<TextInputState>,
    pub(super) output_manager: Option<OutputManagerState>,
    pub(super) seat: SeatState,
    pub(super) xdg: XdgShell,
    pub(super) windows: RefCell<HashMap<usize, Rc<RefCell<WaylandWindowInner>>>>,

    pub(super) active_surface_id: RefCell<Option<ObjectId>>,
    pub(super) last_serial: RefCell<u32>,
    pub(super) keyboard: Option<WlKeyboard>,
    pub(super) keyboard_mapper: Option<KeyboardWithFallback>,
    pub(super) key_repeat_delay: i32,
    pub(super) key_repeat_rate: i32,
    pub(super) keyboard_window_id: Option<usize>,

    pub(super) pointer: Option<ThemedPointer<PointerUserData>>,
    pub(super) surface_to_pending: HashMap<ObjectId, Arc<Mutex<PendingMouse>>>,

    pub(super) data_device_manager_state: DataDeviceManagerState,
    pub(super) data_device: Option<DataDevice>,
    pub(super) copy_paste_source: Option<(CopyPasteSource, String)>,
    pub(super) primary_selection_manager: Option<PrimarySelectionManagerState>,
    pub(super) primary_selection_device: Option<PrimarySelectionDevice>,
    pub(super) primary_selection_source: Option<(PrimarySelectionSource, String)>,
    pub(super) shm: Shm,
    pub(super) mem_pool: RefCell<SlotPool>,
    pub(super) kde_blur_manager: Option<OrgKdeKwinBlurManager>,
    pub(super) presentation: Option<WpPresentation>,
    pub(super) input_timestamps_manager: Option<ZwpInputTimestampsManagerV1>,
    pub(super) commit_timing_manager: Option<WpCommitTimingManagerV1>,
    pub(super) drm_syncobj_manager: Option<WpLinuxDrmSyncobjManagerV1>,
    pub(super) fractional_scale_manager: Option<WpFractionalScaleManagerV1>,
    pub(super) viewporter: Option<WpViewporter>,
    pub(super) tearing_control_manager: Option<WpTearingControlManagerV1>,
}

impl WaylandState {
    pub(super) fn new(globals: &GlobalList, qh: &QueueHandle<Self>) -> anyhow::Result<Self> {
        let shm = Shm::bind(&globals, qh)?;
        let mem_pool = SlotPool::new(1, &shm)?;

        let compositor = CompositorState::bind(globals, qh)?;
        let subcompositor =
            SubcompositorState::bind(compositor.wl_compositor().clone(), globals, qh)?;

        let blur_manager: Option<OrgKdeKwinBlurManager> = globals.bind(qh, 1..=1, GlobalData).ok();
        let presentation: Option<WpPresentation> = globals.bind(qh, 1..=1, GlobalData).ok();
        let input_timestamps_manager: Option<ZwpInputTimestampsManagerV1> = globals.bind(qh, 1..=1, GlobalData).ok();
        let commit_timing_manager: Option<WpCommitTimingManagerV1> = globals.bind(qh, 1..=1, GlobalData).ok();
        let drm_syncobj_manager: Option<WpLinuxDrmSyncobjManagerV1> = globals.bind(qh, 1..=1, GlobalData).ok();
        let fractional_scale_manager: Option<WpFractionalScaleManagerV1> = globals.bind(qh, 1..=1, GlobalData).ok();
        let viewporter: Option<WpViewporter> = globals.bind(qh, 1..=1, GlobalData).ok();
        let tearing_control_manager: Option<WpTearingControlManagerV1> = globals.bind(qh, 1..=1, GlobalData).ok();

        if presentation.is_some() {
            log::info!("wp_presentation protocol available - enabling presentation timing");
        } else {
            log::warn!("wp_presentation protocol not available - presentation timing disabled");
        }

        if input_timestamps_manager.is_some() {
            log::info!("zwp_input_timestamps_v1 protocol available - enabling high-resolution input timestamps");
        } else {
            log::warn!("zwp_input_timestamps_v1 protocol not available - using standard timestamps");
        }

        if commit_timing_manager.is_some() {
            log::info!("wp_commit_timing_v1 protocol available - enabling precise frame timing control");
        } else {
            log::warn!("wp_commit_timing_v1 protocol not available - using default timing");
        }

        if drm_syncobj_manager.is_some() {
            log::info!("wp_linux_drm_syncobj_v1 protocol available - enabling explicit GPU synchronization");
        } else {
            log::warn!("wp_linux_drm_syncobj_v1 protocol not available - using implicit synchronization");
        }

        if fractional_scale_manager.is_some() {
            log::info!("wp_fractional_scale_v1 protocol available - enabling fractional scaling");
        } else {
            log::warn!("wp_fractional_scale_v1 protocol not available - using integer scaling");
        }

        if viewporter.is_some() {
            log::info!("wp_viewporter protocol available - enabling efficient surface scaling");
        } else {
            log::warn!("wp_viewporter protocol not available - scaling disabled");
        }

        if tearing_control_manager.is_some() {
            log::info!("wp_tearing_control_v1 protocol available - can enable low-latency async presentation");
        } else {
            log::warn!("wp_tearing_control_v1 protocol not available - vsync-only presentation");
        }

        let wayland_state = WaylandState {
            registry: RegistryState::new(globals),
            output: OutputState::new(globals, qh),
            compositor,
            subcompositor: Arc::new(subcompositor),
            text_input: TextInputState::bind(globals, qh).ok(),
            output_manager: if config::configuration().enable_zwlr_output_manager {
                Some(OutputManagerState::bind(globals, qh)?)
            } else {
                None
            },
            windows: RefCell::new(HashMap::new()),
            seat: SeatState::new(globals, qh),
            xdg: XdgShell::bind(globals, qh)?,
            active_surface_id: RefCell::new(None),
            last_serial: RefCell::new(0),
            keyboard: None,
            keyboard_mapper: None,
            key_repeat_rate: 25,
            key_repeat_delay: 400,
            keyboard_window_id: None,
            pointer: None,
            surface_to_pending: HashMap::new(),
            data_device_manager_state: DataDeviceManagerState::bind(globals, qh)?,
            data_device: None,
            copy_paste_source: None,
            primary_selection_manager: PrimarySelectionManagerState::bind(globals, qh).ok(),
            primary_selection_device: None,
            primary_selection_source: None,
            shm,
            mem_pool: RefCell::new(mem_pool),
            kde_blur_manager: blur_manager,
            presentation,
            input_timestamps_manager,
            commit_timing_manager,
            drm_syncobj_manager,
            fractional_scale_manager,
            viewporter,
            tearing_control_manager,
        };
        Ok(wayland_state)
    }
}

impl ProvidesRegistryState for WaylandState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }

    registry_handlers![OutputState, SeatState];
}

impl ShmHandler for WaylandState {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl OutputHandler for WaylandState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output
    }

    fn new_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: WlOutput) {
        log::trace!("new output: OutputHandler");
    }

    fn update_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: WlOutput) {
        log::trace!("update output: OutputHandler");
    }

    fn output_destroyed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: WlOutput) {
        log::trace!("output destroyed: OutputHandler");
    }
}

delegate_registry!(WaylandState);

delegate_shm!(WaylandState);

delegate_output!(WaylandState);
delegate_compositor!(WaylandState, surface: [SurfaceData, SurfaceUserData]);
delegate_subcompositor!(WaylandState);

delegate_seat!(WaylandState);

delegate_data_device!(WaylandState);

delegate_pointer!(WaylandState, pointer: [PointerUserData]);

delegate_xdg_shell!(WaylandState);
delegate_xdg_window!(WaylandState);

delegate_primary_selection!(WaylandState);

delegate_dispatch!(WaylandState: [ZwpTextInputManagerV3: GlobalData] => TextInputState);
delegate_dispatch!(WaylandState: [ZwpTextInputV3: TextInputData] => TextInputState);

delegate_dispatch!(WaylandState: [ZwlrOutputManagerV1: GlobalData] => OutputManagerState);
delegate_dispatch!(WaylandState: [ZwlrOutputHeadV1: OutputManagerData] => OutputManagerState);
delegate_dispatch!(WaylandState: [ZwlrOutputModeV1: OutputManagerData] => OutputManagerState);

// Input timestamps event handlers
use wayland_client::protocol::wl_pointer::WlPointer;
use wayland_protocols::wp::input_timestamps::zv1::client::zwp_input_timestamps_v1::Event as InputTimestampEvent;

impl Dispatch<ZwpInputTimestampsManagerV1, GlobalData> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwpInputTimestampsManagerV1,
        _event: <ZwpInputTimestampsManagerV1 as wayland_client::Proxy>::Event,
        _data: &GlobalData,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        // Manager has no events
    }
}

impl Dispatch<ZwpInputTimestampsV1, WlKeyboard> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwpInputTimestampsV1,
        event: <ZwpInputTimestampsV1 as wayland_client::Proxy>::Event,
        _data: &WlKeyboard,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            InputTimestampEvent::Timestamp { tv_sec_hi, tv_sec_lo, tv_nsec } => {
                let tv_sec = ((tv_sec_hi as u64) << 32) | (tv_sec_lo as u64);
                let timestamp_ns = tv_sec * 1_000_000_000 + tv_nsec as u64;
                log::trace!("Keyboard input timestamp: {}ns", timestamp_ns);
                // TODO: Store this timestamp and use it to calculate input latency
                // when combined with presentation feedback timestamps
            }
            _ => {}
        }
    }
}

impl Dispatch<ZwpInputTimestampsV1, WlPointer> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwpInputTimestampsV1,
        event: <ZwpInputTimestampsV1 as wayland_client::Proxy>::Event,
        _data: &WlPointer,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            InputTimestampEvent::Timestamp { tv_sec_hi, tv_sec_lo, tv_nsec } => {
                let tv_sec = ((tv_sec_hi as u64) << 32) | (tv_sec_lo as u64);
                let timestamp_ns = tv_sec * 1_000_000_000 + tv_nsec as u64;
                log::trace!("Pointer input timestamp: {}ns", timestamp_ns);
                // TODO: Store this timestamp and use it to calculate input latency
                // when combined with presentation feedback timestamps
            }
            _ => {}
        }
    }
}

// Fractional scale event handlers
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1::Event as FractionalScaleEvent;

impl Dispatch<WpFractionalScaleManagerV1, GlobalData> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &WpFractionalScaleManagerV1,
        _event: <WpFractionalScaleManagerV1 as wayland_client::Proxy>::Event,
        _data: &GlobalData,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        // Manager has no events
    }
}

impl Dispatch<WpFractionalScaleV1, WlSurface> for WaylandState {
    fn event(
        state: &mut Self,
        _proxy: &WpFractionalScaleV1,
        event: <WpFractionalScaleV1 as wayland_client::Proxy>::Event,
        surface: &WlSurface,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            FractionalScaleEvent::PreferredScale { scale } => {
                // scale is in 120ths, so 120 = 1.0x, 180 = 1.5x, 240 = 2.0x
                let scale_factor = scale as f64 / 120.0;
                log::info!("Fractional scale preferred: {:.2}x ({})", scale_factor, scale);

                // Update the window's fractional scale
                let surface_id = surface.id();
                for window in state.windows.borrow().values() {
                    let mut inner = window.borrow_mut();
                    if inner.surface().id() == surface_id {
                        inner.current_fractional_scale = Some(scale);
                        log::info!("Applied fractional scale {:.2}x to window", scale_factor);
                        // The surface will be resized on next configure event
                        break;
                    }
                }
            }
            _ => {}
        }
    }
}

// Viewporter event handlers
impl Dispatch<WpViewporter, GlobalData> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &WpViewporter,
        _event: <WpViewporter as wayland_client::Proxy>::Event,
        _data: &GlobalData,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        // Viewporter has no events
    }
}

impl Dispatch<WpViewport, WlSurface> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &WpViewport,
        _event: <WpViewport as wayland_client::Proxy>::Event,
        _surface: &WlSurface,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        // Viewport has no events - used for set_source/set_destination requests
    }
}

// DRM Syncobj event handlers (for explicit GPU synchronization)
impl Dispatch<WpLinuxDrmSyncobjManagerV1, GlobalData> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &WpLinuxDrmSyncobjManagerV1,
        _event: <WpLinuxDrmSyncobjManagerV1 as wayland_client::Proxy>::Event,
        _data: &GlobalData,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        // Manager has no events
    }
}

impl Dispatch<WpLinuxDrmSyncobjSurfaceV1, WlSurface> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &WpLinuxDrmSyncobjSurfaceV1,
        _event: <WpLinuxDrmSyncobjSurfaceV1 as wayland_client::Proxy>::Event,
        _surface: &WlSurface,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        // Surface has no events - used for set_acquire_point/set_release_point requests
    }
}

impl Dispatch<WpLinuxDrmSyncobjTimelineV1, GlobalData> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &WpLinuxDrmSyncobjTimelineV1,
        _event: <WpLinuxDrmSyncobjTimelineV1 as wayland_client::Proxy>::Event,
        _data: &GlobalData,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        // Timeline has no events - used for buffer synchronization point tracking
    }
}

// Commit timing event handlers
impl Dispatch<WpCommitTimingManagerV1, GlobalData> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &WpCommitTimingManagerV1,
        _event: <WpCommitTimingManagerV1 as wayland_client::Proxy>::Event,
        _data: &GlobalData,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        // Manager has no events
    }
}

impl Dispatch<WpCommitTimerV1, WlSurface> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &WpCommitTimerV1,
        _event: <WpCommitTimerV1 as wayland_client::Proxy>::Event,
        _surface: &WlSurface,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        // wp_commit_timer_v1 has no events - it only has the set_timestamp request
        // which is used to tell the compositor when content should be presented
    }
}

// Presentation timing event handlers
use wayland_client::{Dispatch, Proxy};
use wayland_protocols::wp::presentation_time::client::wp_presentation_feedback::{Event as PresentationEvent, Kind};

impl Dispatch<WpPresentation, GlobalData> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &WpPresentation,
        _event: <WpPresentation as wayland_client::Proxy>::Event,
        _data: &GlobalData,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        // wp_presentation has only one event: clk_id, which we don't need to handle
    }
}

impl Dispatch<WpPresentationFeedback, WlSurface> for WaylandState {
    fn event(
        state: &mut Self,
        _proxy: &WpPresentationFeedback,
        event: <WpPresentationFeedback as wayland_client::Proxy>::Event,
        surface: &WlSurface,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            PresentationEvent::SyncOutput { .. } => {
                // Indicates which output the surface was presented on
                log::trace!("presentation sync_output");
            }
            PresentationEvent::Presented {
                tv_sec_hi,
                tv_sec_lo,
                tv_nsec,
                refresh,
                seq_hi,
                seq_lo,
                flags,
            } => {
                // Combine high and low parts of timestamp
                let tv_sec = ((tv_sec_hi as u64) << 32) | (tv_sec_lo as u64);
                let presentation_time_ns = tv_sec * 1_000_000_000 + tv_nsec as u64;

                // Check flags for presentation characteristics
                // flags is a WEnum, convert to Kind bitflags
                let kind_flags = Kind::from_bits_truncate(flags.into());
                let vsync = kind_flags.contains(Kind::Vsync);
                let hw_clock = kind_flags.contains(Kind::HwClock);
                let hw_completion = kind_flags.contains(Kind::HwCompletion);
                let zero_copy = kind_flags.contains(Kind::ZeroCopy);

                // Combine MSC (Media Stream Counter) from high and low parts
                let msc = ((seq_hi as u64) << 32) | (seq_lo as u64);

                log::info!(
                    "presentation feedback: time={}ns, refresh={}ns, msc={}, vsync={}, hw_clock={}, hw_completion={}, zero_copy={}",
                    presentation_time_ns,
                    refresh,
                    msc,
                    vsync,
                    hw_clock,
                    hw_completion,
                    zero_copy
                );

                // Update the last presentation time for the window
                let surface_id = surface.id();
                for window in state.windows.borrow().values() {
                    let mut inner = window.borrow_mut();
                    if inner.surface().id() == surface_id {
                        inner.last_presentation_time = Some(presentation_time_ns);
                        break;
                    }
                }
            }
            PresentationEvent::Discarded => {
                log::trace!("presentation feedback discarded");
            }
            _ => {}
        }
    }
}

// Tearing control event handlers
impl Dispatch<WpTearingControlManagerV1, GlobalData> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &WpTearingControlManagerV1,
        _event: <WpTearingControlManagerV1 as wayland_client::Proxy>::Event,
        _data: &GlobalData,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        // Manager has no events
    }
}

impl Dispatch<WpTearingControlV1, WlSurface> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &WpTearingControlV1,
        _event: <WpTearingControlV1 as wayland_client::Proxy>::Event,
        _surface: &WlSurface,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        // Tearing control has no events - only set_presentation_hint request
        // Default is vsync. Can be set to async for low-latency with tearing acceptable.
    }
}
