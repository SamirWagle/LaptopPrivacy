use serde::Serialize;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BrightnessDisplay {
    pub id: String,
    pub name: String,
    pub brightness_percent: u8,
    pub built_in: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BrightnessStatus {
    pub supported: bool,
    pub displays: Vec<BrightnessDisplay>,
    pub message: String,
}

pub(crate) use platform::Snapshot;

#[derive(Clone, Default)]
pub struct BrightnessControl(Arc<ControlInner>);

#[derive(Default)]
struct ControlInner {
    generation: AtomicU64,
    state: Mutex<ControlState>,
}

#[derive(Default)]
struct ControlState {
    snapshot: Option<Snapshot>,
    mode: Option<ControlMode>,
}

enum ControlMode {
    Automatic { rule_id: String, percent: u8 },
    Manual,
    Preview(u64),
}

impl BrightnessControl {
    pub fn preview(&self, percent: u8) -> Result<BrightnessStatus, String> {
        let generation = self.0.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let status = self.begin(percent, ControlMode::Preview(generation))?;
        let control = self.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(3));
            let _ = control.restore_preview(generation);
        });
        Ok(BrightnessStatus {
            message: "Hardware brightness preview active for three seconds".into(),
            ..status
        })
    }

    pub fn apply_manual(&self, percent: u8) -> Result<BrightnessStatus, String> {
        self.0.generation.fetch_add(1, Ordering::SeqCst);
        self.begin(percent, ControlMode::Manual)
    }

    pub fn reconcile_automatic(&self, rule_id: String, percent: u8) -> Result<bool, String> {
        validate_percent(percent)?;
        let mut state = self
            .0
            .state
            .lock()
            .map_err(|_| "brightness state is unavailable")?;
        match state.mode.as_ref() {
            Some(ControlMode::Preview(_)) | Some(ControlMode::Manual) => return Ok(false),
            Some(ControlMode::Automatic {
                rule_id: current_rule,
                percent: current_percent,
            }) if current_rule == &rule_id && *current_percent == percent => return Ok(true),
            _ => {}
        }
        if let Some(snapshot) = state.snapshot.as_ref() {
            set(snapshot, percent)?;
        } else {
            let (_, snapshot) = apply(percent)?;
            state.snapshot = Some(snapshot);
        }
        state.mode = Some(ControlMode::Automatic { rule_id, percent });
        Ok(true)
    }

    pub fn clear_automatic(&self) -> Result<bool, String> {
        let mut state = self
            .0
            .state
            .lock()
            .map_err(|_| "brightness state is unavailable")?;
        if matches!(state.mode, Some(ControlMode::Automatic { .. })) {
            restore_state(&mut state)?;
        }
        Ok(false)
    }

    pub fn cancel(&self) -> Result<(), String> {
        self.0.generation.fetch_add(1, Ordering::SeqCst);
        let mut state = self
            .0
            .state
            .lock()
            .map_err(|_| "brightness state is unavailable")?;
        restore_state(&mut state)
    }

    fn begin(&self, percent: u8, mode: ControlMode) -> Result<BrightnessStatus, String> {
        validate_percent(percent)?;
        let mut state = self
            .0
            .state
            .lock()
            .map_err(|_| "brightness state is unavailable")?;
        restore_state(&mut state)?;
        let (status, snapshot) = apply(percent)?;
        state.snapshot = Some(snapshot);
        state.mode = Some(mode);
        Ok(status)
    }

    fn restore_preview(&self, generation: u64) -> Result<(), String> {
        if self.0.generation.load(Ordering::SeqCst) != generation {
            return Ok(());
        }
        let mut state = self
            .0
            .state
            .lock()
            .map_err(|_| "brightness state is unavailable")?;
        if matches!(state.mode, Some(ControlMode::Preview(value)) if value == generation) {
            restore_state(&mut state)?;
        }
        Ok(())
    }
}

fn restore_state(state: &mut ControlState) -> Result<(), String> {
    if let Some(snapshot) = state.snapshot.as_ref() {
        restore(snapshot)?;
    }
    state.snapshot = None;
    state.mode = None;
    Ok(())
}

pub fn status() -> BrightnessStatus {
    match platform::snapshot() {
        Ok(snapshot) if !snapshot.displays().is_empty() => BrightnessStatus {
            supported: true,
            displays: snapshot.displays(),
            message: "Hardware brightness control available".into(),
        },
        Ok(_) => unsupported("No controllable display brightness interface found"),
        Err(error) => unsupported(&error),
    }
}

pub(crate) fn apply(percent: u8) -> Result<(BrightnessStatus, Snapshot), String> {
    validate_percent(percent)?;
    let snapshot = platform::snapshot()?;
    if snapshot.displays().is_empty() {
        return Err("No controllable display brightness interface found".into());
    }
    platform::set(&snapshot, percent)?;
    let displays = snapshot
        .displays()
        .into_iter()
        .map(|mut display| {
            display.brightness_percent = percent;
            display
        })
        .collect();
    Ok((
        BrightnessStatus {
            supported: true,
            displays,
            message: "Hardware brightness applied".into(),
        },
        snapshot,
    ))
}

fn set(snapshot: &Snapshot, percent: u8) -> Result<BrightnessStatus, String> {
    validate_percent(percent)?;
    platform::set(snapshot, percent)?;
    let displays = snapshot
        .displays()
        .into_iter()
        .map(|mut display| {
            display.brightness_percent = percent;
            display
        })
        .collect();
    Ok(BrightnessStatus {
        supported: true,
        displays,
        message: "Hardware brightness applied".into(),
    })
}

pub(crate) fn restore(snapshot: &Snapshot) -> Result<(), String> {
    platform::restore(snapshot)
}

fn validate_percent(percent: u8) -> Result<(), String> {
    if (10..=100).contains(&percent) {
        Ok(())
    } else {
        Err("hardware brightness must be between 10 and 100".into())
    }
}

fn unsupported(message: &str) -> BrightnessStatus {
    BrightnessStatus {
        supported: false,
        displays: Vec::new(),
        message: message.into(),
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::BrightnessDisplay;
    use std::ffi::{c_char, c_void};

    const SUCCESS: i32 = 0;
    const UTF8_ENCODING: u32 = 0x0800_0100;
    const MAX_DISPLAYS: usize = 16;

    type DisplayId = u32;
    type IoService = u32;
    type CfStringRef = *const c_void;

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGGetActiveDisplayList(
            max_displays: u32,
            displays: *mut DisplayId,
            count: *mut u32,
        ) -> i32;
        fn CGMainDisplayID() -> DisplayId;
        fn CGDisplayIOServicePort(display: DisplayId) -> IoService;
        fn CGDisplayIsBuiltin(display: DisplayId) -> u32;
    }

    #[link(name = "IOKit", kind = "framework")]
    extern "C" {
        fn IODisplayGetFloatParameter(
            service: IoService,
            options: u32,
            parameter_name: CfStringRef,
            value: *mut f32,
        ) -> i32;
        fn IODisplaySetFloatParameter(
            service: IoService,
            options: u32,
            parameter_name: CfStringRef,
            value: f32,
        ) -> i32;
    }

    #[link(name = "DisplayServices", kind = "framework")]
    extern "C" {
        fn DisplayServicesGetBrightness(display: DisplayId, brightness: *mut f32) -> i32;
        fn DisplayServicesSetBrightness(display: DisplayId, brightness: f32) -> i32;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFStringCreateWithCString(
            allocator: *const c_void,
            value: *const c_char,
            encoding: u32,
        ) -> CfStringRef;
        fn CFRelease(value: *const c_void);
    }

    #[derive(Clone, Debug)]
    enum Backend {
        IoKit,
        DisplayServices,
    }

    #[derive(Clone, Debug)]
    struct Reading {
        display_id: DisplayId,
        service: IoService,
        brightness: f32,
        built_in: bool,
        backend: Backend,
    }

    #[derive(Clone, Debug)]
    pub struct Snapshot(Vec<Reading>);

    impl Snapshot {
        pub fn displays(&self) -> Vec<BrightnessDisplay> {
            self.0
                .iter()
                .map(|reading| BrightnessDisplay {
                    id: reading.display_id.to_string(),
                    name: if reading.built_in {
                        "Built-in display".into()
                    } else {
                        format!("External display {}", reading.display_id)
                    },
                    brightness_percent: (reading.brightness.clamp(0.0, 1.0) * 100.0).round() as u8,
                    built_in: reading.built_in,
                })
                .collect()
        }
    }

    struct BrightnessKey(CfStringRef);

    impl BrightnessKey {
        fn new() -> Result<Self, String> {
            let key = unsafe {
                CFStringCreateWithCString(std::ptr::null(), c"brightness".as_ptr(), UTF8_ENCODING)
            };
            if key.is_null() {
                Err("Could not create brightness parameter key".into())
            } else {
                Ok(Self(key))
            }
        }
    }

    impl Drop for BrightnessKey {
        fn drop(&mut self) {
            unsafe { CFRelease(self.0) }
        }
    }

    pub fn snapshot() -> Result<Snapshot, String> {
        let mut ids = [0; MAX_DISPLAYS];
        let mut count = 0;
        let result =
            unsafe { CGGetActiveDisplayList(MAX_DISPLAYS as u32, ids.as_mut_ptr(), &mut count) };
        if result != SUCCESS {
            return Err(format!(
                "Could not enumerate displays: CoreGraphics error {result}"
            ));
        }
        if count == 0 {
            ids[0] = unsafe { CGMainDisplayID() };
            count = 1;
        }
        let key = BrightnessKey::new()?;
        let mut readings = Vec::new();
        let mut failures = Vec::new();
        for display_id in &ids[..count as usize] {
            let service = unsafe { CGDisplayIOServicePort(*display_id) };
            let mut brightness = 0.0;
            let iokit_result =
                unsafe { IODisplayGetFloatParameter(service, 0, key.0, &mut brightness) };
            let built_in = unsafe { CGDisplayIsBuiltin(*display_id) != 0 };
            if iokit_result == SUCCESS && brightness.is_finite() {
                readings.push(Reading {
                    display_id: *display_id,
                    service,
                    brightness,
                    built_in,
                    backend: Backend::IoKit,
                });
                continue;
            }
            let display_services_result = if built_in {
                unsafe { DisplayServicesGetBrightness(*display_id, &mut brightness) }
            } else {
                -1
            };
            if display_services_result == SUCCESS && brightness.is_finite() {
                readings.push(Reading {
                    display_id: *display_id,
                    service,
                    brightness,
                    built_in,
                    backend: Backend::DisplayServices,
                });
            } else {
                failures.push(format!(
                    "{} (built_in={built_in}, IOKit={iokit_result}, DisplayServices={display_services_result})",
                    display_id
                ));
            }
        }
        if readings.is_empty() && !failures.is_empty() {
            Err(format!("No controllable display: {}", failures.join(", ")))
        } else {
            Ok(Snapshot(readings))
        }
    }

    pub fn set(snapshot: &Snapshot, percent: u8) -> Result<(), String> {
        let key = BrightnessKey::new()?;
        let value = f32::from(percent) / 100.0;
        for (index, reading) in snapshot.0.iter().enumerate() {
            let result = set_reading(reading, key.0, value);
            if result != SUCCESS {
                for changed in &snapshot.0[..index] {
                    set_reading(changed, key.0, changed.brightness);
                }
                return Err(format!(
                    "Display {} rejected hardware brightness: IOKit error {result}",
                    reading.display_id
                ));
            }
        }
        Ok(())
    }

    pub fn restore(snapshot: &Snapshot) -> Result<(), String> {
        let key = BrightnessKey::new()?;
        let failures: Vec<_> = snapshot
            .0
            .iter()
            .filter_map(|reading| {
                let result = set_reading(reading, key.0, reading.brightness);
                (result != SUCCESS).then_some(reading.display_id)
            })
            .collect();
        if failures.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "Could not restore display brightness for {failures:?}"
            ))
        }
    }

    fn set_reading(reading: &Reading, key: CfStringRef, value: f32) -> i32 {
        unsafe {
            match reading.backend {
                Backend::IoKit => IODisplaySetFloatParameter(reading.service, 0, key, value),
                Backend::DisplayServices => DisplayServicesSetBrightness(reading.display_id, value),
            }
        }
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use super::BrightnessDisplay;
    use std::ffi::c_void;

    type Handle = *mut c_void;
    type MonitorHandle = *mut c_void;
    type DeviceContext = *mut c_void;

    #[repr(C)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[repr(C)]
    struct PhysicalMonitor {
        handle: Handle,
        description: [u16; 128],
    }

    #[link(name = "User32")]
    extern "system" {
        fn EnumDisplayMonitors(
            hdc: DeviceContext,
            clip: *const Rect,
            callback: unsafe extern "system" fn(
                MonitorHandle,
                DeviceContext,
                *mut Rect,
                isize,
            ) -> i32,
            data: isize,
        ) -> i32;
    }

    #[link(name = "Dxva2")]
    extern "system" {
        fn GetNumberOfPhysicalMonitorsFromHMONITOR(monitor: MonitorHandle, count: *mut u32) -> i32;
        fn GetPhysicalMonitorsFromHMONITOR(
            monitor: MonitorHandle,
            count: u32,
            physical_monitors: *mut PhysicalMonitor,
        ) -> i32;
        fn DestroyPhysicalMonitors(count: u32, physical_monitors: *mut PhysicalMonitor) -> i32;
        fn GetMonitorBrightness(
            monitor: Handle,
            min: *mut u32,
            current: *mut u32,
            max: *mut u32,
        ) -> i32;
        fn SetMonitorBrightness(monitor: Handle, brightness: u32) -> i32;
    }

    #[derive(Clone, Debug)]
    struct Reading {
        id: String,
        name: String,
        current: u32,
        min: u32,
        max: u32,
    }

    struct NativeMonitor {
        handle: Handle,
        reading: Option<Reading>,
    }

    #[derive(Clone, Debug)]
    pub struct Snapshot(Vec<Reading>);

    impl Snapshot {
        pub fn displays(&self) -> Vec<BrightnessDisplay> {
            self.0
                .iter()
                .map(|reading| BrightnessDisplay {
                    id: reading.id.clone(),
                    name: reading.name.clone(),
                    brightness_percent: to_percent(reading.current, reading.min, reading.max),
                    built_in: false,
                })
                .collect()
        }
    }

    unsafe extern "system" fn collect_monitor(
        monitor: MonitorHandle,
        _: DeviceContext,
        _: *mut Rect,
        data: isize,
    ) -> i32 {
        (*(data as *mut Vec<MonitorHandle>)).push(monitor);
        1
    }

    fn enumerate() -> Result<Vec<NativeMonitor>, String> {
        let mut logical = Vec::new();
        if unsafe {
            EnumDisplayMonitors(
                std::ptr::null_mut(),
                std::ptr::null(),
                collect_monitor,
                (&mut logical as *mut Vec<MonitorHandle>) as isize,
            )
        } == 0
        {
            return Err("Windows could not enumerate monitors".into());
        }
        let mut readings = Vec::new();
        for (logical_index, monitor) in logical.into_iter().enumerate() {
            let mut count = 0;
            if unsafe { GetNumberOfPhysicalMonitorsFromHMONITOR(monitor, &mut count) } == 0
                || count == 0
            {
                continue;
            }
            let mut physical: Vec<PhysicalMonitor> = (0..count)
                .map(|_| PhysicalMonitor {
                    handle: std::ptr::null_mut(),
                    description: [0; 128],
                })
                .collect();
            if unsafe { GetPhysicalMonitorsFromHMONITOR(monitor, count, physical.as_mut_ptr()) }
                == 0
            {
                continue;
            }
            for (physical_index, item) in physical.iter().enumerate() {
                let (mut min, mut current, mut max) = (0, 0, 0);
                let reading = if unsafe {
                    GetMonitorBrightness(item.handle, &mut min, &mut current, &mut max)
                } != 0
                    && max > min
                {
                    let end = item
                        .description
                        .iter()
                        .position(|value| *value == 0)
                        .unwrap_or(128);
                    Some(Reading {
                        id: format!("{logical_index}-{physical_index}"),
                        name: String::from_utf16_lossy(&item.description[..end]),
                        current,
                        min,
                        max,
                    })
                } else {
                    None
                };
                readings.push(NativeMonitor {
                    handle: item.handle,
                    reading,
                });
            }
        }
        Ok(readings)
    }

    fn close(monitors: &[NativeMonitor]) {
        for native in monitors {
            let mut monitor = PhysicalMonitor {
                handle: native.handle,
                description: [0; 128],
            };
            unsafe { DestroyPhysicalMonitors(1, &mut monitor) };
        }
    }

    fn to_percent(value: u32, min: u32, max: u32) -> u8 {
        (((value - min) as f64 / (max - min) as f64) * 100.0).round() as u8
    }

    pub fn snapshot() -> Result<Snapshot, String> {
        let monitors = enumerate()?;
        let snapshot = Snapshot(
            monitors
                .iter()
                .filter_map(|native| native.reading.clone())
                .collect(),
        );
        close(&monitors);
        Ok(snapshot)
    }

    pub fn set(snapshot: &Snapshot, percent: u8) -> Result<(), String> {
        let monitors = enumerate()?;
        let mut error = None;
        for native in &monitors {
            if let Some(reading) = &native.reading {
                if snapshot.0.iter().any(|saved| saved.id == reading.id) {
                    let target =
                        reading.min + ((reading.max - reading.min) * u32::from(percent) / 100);
                    if unsafe { SetMonitorBrightness(native.handle, target) } == 0 {
                        error = Some(format!(
                            "Monitor {} rejected hardware brightness",
                            reading.name
                        ));
                        break;
                    }
                }
            }
        }
        close(&monitors);
        error.map_or(Ok(()), Err)
    }

    pub fn restore(snapshot: &Snapshot) -> Result<(), String> {
        let monitors = enumerate()?;
        let mut failures = Vec::new();
        for native in &monitors {
            if let Some(reading) = &native.reading {
                if let Some(saved) = snapshot.0.iter().find(|saved| saved.id == reading.id) {
                    if unsafe { SetMonitorBrightness(native.handle, saved.current) } == 0 {
                        failures.push(reading.name.clone());
                    }
                }
            }
        }
        close(&monitors);
        if failures.is_empty() {
            Ok(())
        } else {
            Err(format!("Could not restore {failures:?}"))
        }
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::BrightnessDisplay;
    use std::{fs, path::PathBuf};

    const BACKLIGHT_ROOT: &str = "/sys/class/backlight";

    #[derive(Clone, Debug)]
    struct Reading {
        id: String,
        path: PathBuf,
        current: u32,
        max: u32,
    }

    #[derive(Clone, Debug)]
    pub struct Snapshot(Vec<Reading>);

    impl Snapshot {
        pub fn displays(&self) -> Vec<BrightnessDisplay> {
            self.0
                .iter()
                .map(|reading| BrightnessDisplay {
                    id: reading.id.clone(),
                    name: reading.id.clone(),
                    brightness_percent: ((reading.current as f64 / reading.max as f64) * 100.0)
                        .round() as u8,
                    built_in: true,
                })
                .collect()
        }
    }

    fn read_number(path: PathBuf) -> Option<u32> {
        fs::read_to_string(path).ok()?.trim().parse().ok()
    }

    pub fn snapshot() -> Result<Snapshot, String> {
        let entries = fs::read_dir(BACKLIGHT_ROOT)
            .map_err(|error| format!("Could not read Linux backlight interfaces: {error}"))?;
        let readings = entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let path = entry.path();
                let max = read_number(path.join("max_brightness"))?;
                let current = read_number(path.join("brightness"))?;
                (max > 0).then(|| Reading {
                    id: entry.file_name().to_string_lossy().into_owned(),
                    path,
                    current,
                    max,
                })
            })
            .collect();
        Ok(Snapshot(readings))
    }

    pub fn set(snapshot: &Snapshot, percent: u8) -> Result<(), String> {
        for reading in &snapshot.0 {
            let target = reading.max * u32::from(percent) / 100;
            if let Err(error) = fs::write(reading.path.join("brightness"), target.to_string()) {
                let _ = restore(snapshot);
                return Err(format!("Could not set {} brightness: {error}", reading.id));
            }
        }
        Ok(())
    }

    pub fn restore(snapshot: &Snapshot) -> Result<(), String> {
        let failures: Vec<_> = snapshot
            .0
            .iter()
            .filter_map(|reading| {
                fs::write(reading.path.join("brightness"), reading.current.to_string())
                    .err()
                    .map(|error| format!("{}: {error}", reading.id))
            })
            .collect();
        if failures.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "Could not restore brightness: {}",
                failures.join(", ")
            ))
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
mod platform {
    use super::BrightnessDisplay;

    #[derive(Clone, Debug)]
    pub struct Snapshot;
    impl Snapshot {
        pub fn displays(&self) -> Vec<BrightnessDisplay> {
            Vec::new()
        }
    }
    pub fn snapshot() -> Result<Snapshot, String> {
        Err("Hardware brightness is unsupported on this platform".into())
    }
    pub fn set(_: &Snapshot, _: u8) -> Result<(), String> {
        Err("Hardware brightness is unsupported on this platform".into())
    }
    pub fn restore(_: &Snapshot) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hardware_bounds_reject_unsafe_values() {
        assert!(validate_percent(9).is_err());
        assert!(validate_percent(10).is_ok());
        assert!(validate_percent(100).is_ok());
    }

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "changes physical display brightness briefly"]
    fn automatic_control_changes_and_restores_panel_level() {
        let original = status().displays[0].brightness_percent;
        assert!(original > 10, "display already at minimum test level");
        let target = original.saturating_sub(10).max(10);
        let control = BrightnessControl::default();
        assert!(control
            .reconcile_automatic("hardware-test".into(), target)
            .unwrap());
        let changed = status().displays[0].brightness_percent;
        assert!(changed.abs_diff(target) <= 2);
        control.cancel().unwrap();
        let restored = status().displays[0].brightness_percent;
        assert!(restored.abs_diff(original) <= 8);
    }
}
