use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ForegroundApplication {
    pub platform_app_id: String,
    pub display_name: String,
}

#[cfg(target_os = "macos")]
mod platform {
    use super::ForegroundApplication;
    use std::ffi::{c_char, c_void, CStr};

    type Object = *mut c_void;
    type Selector = *mut c_void;

    #[link(name = "AppKit", kind = "framework")]
    extern "C" {}

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
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use super::ForegroundApplication;

    pub fn current() -> Result<Option<ForegroundApplication>, String> {
        Ok(None)
    }

    pub fn running() -> Result<Vec<ForegroundApplication>, String> {
        Ok(Vec::new())
    }
}

pub fn current() -> Result<Option<ForegroundApplication>, String> {
    platform::current()
}

pub fn running() -> Result<Vec<ForegroundApplication>, String> {
    platform::running()
}

pub const fn supported() -> bool {
    cfg!(target_os = "macos")
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires an active macOS desktop session"]
    fn reads_foreground_and_running_applications() {
        let app = current().unwrap().expect("foreground app should exist");
        assert!(!app.platform_app_id.is_empty());
        assert!(!app.display_name.is_empty());
        assert!(!running().unwrap().is_empty());
    }
}
