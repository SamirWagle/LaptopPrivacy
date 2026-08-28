use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ForegroundApplication {
    pub platform_app_id: String,
    pub display_name: String,
    #[serde(skip_serializing)]
    pub process_id: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WindowBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[cfg(target_os = "macos")]
mod platform {
    use super::{ForegroundApplication, WindowBounds};
    use std::ffi::{c_char, c_void, CStr};

    type Object = *mut c_void;
    type Selector = *mut c_void;

    #[link(name = "AppKit", kind = "framework")]
    extern "C" {}

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFRelease(value: *const c_void);
    }

    #[link(name = "objc")]
    extern "C" {
        fn objc_getClass(name: *const c_char) -> Object;
        fn sel_registerName(name: *const c_char) -> Selector;
        fn objc_msgSend();
    }

    unsafe fn selector(name: &CStr) -> Selector {
        sel_registerName(name.as_ptr())
    }

    unsafe fn send_object(receiver: Object, name: &CStr) -> Object {
        let function: unsafe extern "C" fn(Object, Selector) -> Object =
            std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector(name))
    }

    unsafe fn send_object_at(receiver: Object, name: &CStr, index: usize) -> Object {
        let function: unsafe extern "C" fn(Object, Selector, usize) -> Object =
            std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector(name), index)
    }

    unsafe fn send_usize(receiver: Object, name: &CStr) -> usize {
        let function: unsafe extern "C" fn(Object, Selector) -> usize =
            std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector(name))
    }

    unsafe fn send_i32(receiver: Object, name: &CStr) -> i32 {
        let function: unsafe extern "C" fn(Object, Selector) -> i32 =
            std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector(name))
    }

    unsafe fn send_f64(receiver: Object, name: &CStr) -> f64 {
        let function: unsafe extern "C" fn(Object, Selector) -> f64 =
            std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector(name))
    }

    unsafe fn send_object_with_object(receiver: Object, name: &CStr, value: Object) -> Object {
        let function: unsafe extern "C" fn(Object, Selector, Object) -> Object =
            std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector(name), value)
    }

    unsafe fn send_void(receiver: Object, name: &CStr) {
        let function: unsafe extern "C" fn(Object, Selector) =
            std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector(name));
    }

    unsafe fn string(value: Object) -> Option<String> {
        if value.is_null() {
            return None;
        }
        let pointer = send_object(value, c"UTF8String").cast::<c_char>();
        (!pointer.is_null()).then(|| CStr::from_ptr(pointer).to_string_lossy().into_owned())
    }

    unsafe fn application(value: Object) -> Option<ForegroundApplication> {
        if value.is_null() {
            return None;
        }
        let platform_app_id = string(send_object(value, c"bundleIdentifier"))?;
        let display_name =
            string(send_object(value, c"localizedName")).unwrap_or_else(|| platform_app_id.clone());
        Some(ForegroundApplication {
            platform_app_id,
            display_name,
            process_id: send_i32(value, c"processIdentifier"),
        })
    }

    unsafe fn with_pool<T>(operation: impl FnOnce() -> T) -> T {
        let pool_class = objc_getClass(c"NSAutoreleasePool".as_ptr());
        let pool = send_object(send_object(pool_class, c"alloc"), c"init");
        let result = operation();
        send_void(pool, c"release");
        result
    }

    pub fn current() -> Result<Option<ForegroundApplication>, String> {
        unsafe {
            with_pool(|| {
                let workspace_class = objc_getClass(c"NSWorkspace".as_ptr());
                if workspace_class.is_null() {
                    return Err("macOS foreground application API is unavailable".into());
                }
                let workspace = send_object(workspace_class, c"sharedWorkspace");
                Ok(application(send_object(workspace, c"frontmostApplication")))
            })
        }
    }

    pub fn running() -> Result<Vec<ForegroundApplication>, String> {
        unsafe {
            with_pool(|| {
                let workspace_class = objc_getClass(c"NSWorkspace".as_ptr());
                if workspace_class.is_null() {
                    return Err("macOS running application API is unavailable".into());
                }
                let workspace = send_object(workspace_class, c"sharedWorkspace");
                let applications = send_object(workspace, c"runningApplications");
                let count = send_usize(applications, c"count");
                let mut by_id = std::collections::BTreeMap::new();
                for index in 0..count {
                    if let Some(app) =
                        application(send_object_at(applications, c"objectAtIndex:", index))
                    {
                        if app.platform_app_id != "com.privacyaperture.desktop" {
                            by_id.entry(app.platform_app_id.clone()).or_insert(app);
                        }
                    }
                }
                Ok(by_id.into_values().collect())
            })
        }
    }

    #[repr(C)]
    #[derive(Default)]
    struct Point {
        x: f64,
        y: f64,
    }

    #[repr(C)]
    #[derive(Default)]
    struct Size {
        width: f64,
        height: f64,
    }

    #[repr(C)]
    #[derive(Default)]
    struct Rect {
        origin: Point,
        size: Size,
    }

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        static kCGWindowOwnerPID: Object;
        static kCGWindowLayer: Object;
        static kCGWindowBounds: Object;
        static kCGWindowAlpha: Object;
        fn CGWindowListCopyWindowInfo(option: u32, relative_to_window: u32) -> Object;
        fn CGRectMakeWithDictionaryRepresentation(dictionary: Object, rect: *mut Rect) -> bool;
    }

    pub fn window_bounds(process_id: i32) -> Result<Vec<WindowBounds>, String> {
        const ON_SCREEN_ONLY: u32 = 1;
        const EXCLUDE_DESKTOP: u32 = 16;
        unsafe {
            let list = CGWindowListCopyWindowInfo(ON_SCREEN_ONLY | EXCLUDE_DESKTOP, 0);
            if list.is_null() {
                return Err("macOS window list is unavailable".into());
            }
            let count = send_usize(list, c"count");
            let mut windows = Vec::new();
            for index in 0..count {
                let info = send_object_at(list, c"objectAtIndex:", index);
                let owner = send_object_with_object(info, c"objectForKey:", kCGWindowOwnerPID);
                let layer = send_object_with_object(info, c"objectForKey:", kCGWindowLayer);
                let alpha = send_object_with_object(info, c"objectForKey:", kCGWindowAlpha);
                if owner.is_null()
                    || send_i32(owner, c"intValue") != process_id
                    || layer.is_null()
                    || send_i32(layer, c"intValue") != 0
                    || (!alpha.is_null() && send_f64(alpha, c"doubleValue") <= 0.0)
                {
                    continue;
                }
                let bounds = send_object_with_object(info, c"objectForKey:", kCGWindowBounds);
                let mut rect = Rect::default();
                if !bounds.is_null()
                    && CGRectMakeWithDictionaryRepresentation(bounds, &mut rect)
                    && rect.size.width >= 40.0
                    && rect.size.height >= 40.0
                {
                    windows.push(WindowBounds {
                        x: rect.origin.x,
                        y: rect.origin.y,
                        width: rect.size.width,
                        height: rect.size.height,
                    });
                }
            }
            CFRelease(list);
            windows.sort_by(|left, right| {
                left.x
                    .total_cmp(&right.x)
                    .then(left.y.total_cmp(&right.y))
                    .then(left.width.total_cmp(&right.width))
                    .then(left.height.total_cmp(&right.height))
            });
            Ok(windows)
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use super::{ForegroundApplication, WindowBounds};

    pub fn current() -> Result<Option<ForegroundApplication>, String> {
        Ok(None)
    }

    pub fn running() -> Result<Vec<ForegroundApplication>, String> {
        Ok(Vec::new())
    }

    pub fn window_bounds(_: i32) -> Result<Vec<WindowBounds>, String> {
        Ok(Vec::new())
    }
}

pub fn current() -> Result<Option<ForegroundApplication>, String> {
    platform::current()
}

pub fn running() -> Result<Vec<ForegroundApplication>, String> {
    platform::running()
}

pub fn window_bounds(process_id: i32) -> Result<Vec<WindowBounds>, String> {
    platform::window_bounds(process_id)
}

pub const fn supported() -> bool {
    cfg!(target_os = "macos")
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn process_id_stays_out_of_ui_payload() {
        let value = serde_json::to_value(ForegroundApplication {
            platform_app_id: "com.example.private".into(),
            display_name: "Private".into(),
            process_id: 42,
        })
        .unwrap();
        assert!(value.get("process_id").is_none());
    }

    #[test]
    #[ignore = "requires an active macOS desktop session"]
    fn reads_foreground_and_running_applications() {
        let app = current().unwrap().expect("foreground app should exist");
        assert!(!app.platform_app_id.is_empty());
        assert!(!app.display_name.is_empty());
        assert!(!running().unwrap().is_empty());
        assert!(!window_bounds(app.process_id).unwrap().is_empty());
    }
}
