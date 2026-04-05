use testapp_core::{App, Item};
use std::sync::{Mutex, OnceLock};
use std::os::raw::{c_char, c_void};
use std::ffi::{CStr, CString};
use std::panic;

static APP: OnceLock<Mutex<App>> = OnceLock::new();

// Set up panic handler to prevent crashes from propagating to C#
fn setup_panic_handler() {
    panic::set_hook(Box::new(|panic_info| {
        eprintln!("Rust panic occurred: {:?}", panic_info);
    }));
}

#[no_mangle]
pub extern "C" fn app_init() {
    // Catch any panics and convert to safe return
    let result = panic::catch_unwind(|| {
        setup_panic_handler();
        APP.get_or_init(|| Mutex::new(App::new()));
    });
    
    if result.is_err() {
        eprintln!("app_init failed with panic");
    }
}

#[no_mangle]
pub extern "C" fn app_add_item(id: *const c_char, title: *const c_char) {
    let _ = panic::catch_unwind(|| {
        if let Some(app_mutex) = APP.get() {
            unsafe {
                let id_str = CStr::from_ptr(id).to_string_lossy().to_string();
                let title_str = CStr::from_ptr(title).to_string_lossy().to_string();
                
                let mut app = app_mutex.lock().unwrap();
                app.add_item(id_str, title_str);
            }
        }
    });
}

#[no_mangle]
pub extern "C" fn app_toggle_item(id: *const c_char) {
    let _ = panic::catch_unwind(|| {
        if let Some(app_mutex) = APP.get() {
            unsafe {
                let id_str = CStr::from_ptr(id).to_string_lossy();
                let mut app = app_mutex.lock().unwrap();
                app.toggle_item(&id_str);
            }
        }
    });
}

#[no_mangle]
pub extern "C" fn app_delete_item(id: *const c_char) {
    let _ = panic::catch_unwind(|| {
        if let Some(app_mutex) = APP.get() {
            unsafe {
                let id_str = CStr::from_ptr(id).to_string_lossy();
                let mut app = app_mutex.lock().unwrap();
                app.delete_item(&id_str);
            }
        }
    });
}

#[repr(C)]
pub struct CItem {
    pub id: *mut c_char,
    pub title: *mut c_char,
    pub completed: bool,
}

#[repr(C)]
pub struct CItemArray {
    pub items: *mut CItem,
    pub len: usize,
}

#[no_mangle]
pub extern "C" fn app_get_items() -> CItemArray {
    match panic::catch_unwind(|| {
        if let Some(app_mutex) = APP.get() {
            let app = app_mutex.lock().unwrap();
            let items = app.get_items();
            
            let mut c_items: Vec<CItem> = items.iter().map(|item| {
                unsafe {
                    CItem {
                        id: CString::new(item.id.clone()).unwrap().into_raw(),
                        title: CString::new(item.title.clone()).unwrap().into_raw(),
                        completed: item.completed,
                    }
                }
            }).collect();
            
            let len = c_items.len();
            let ptr = c_items.as_mut_ptr();
            std::mem::forget(c_items);
            
            CItemArray { items: ptr, len }
        } else {
            CItemArray { items: std::ptr::null_mut(), len: 0 }
        }
    }) {
        Ok(result) => result,
        Err(_) => CItemArray { items: std::ptr::null_mut(), len: 0 }
    }
}

#[no_mangle]
pub extern "C" fn app_free_items(array: CItemArray) {
    let _ = panic::catch_unwind(|| {
        unsafe {
            if !array.items.is_null() {
                let items = Vec::from_raw_parts(array.items, array.len, array.len);
                for item in items {
                    if !item.id.is_null() {
                        let _ = CString::from_raw(item.id);
                    }
                    if !item.title.is_null() {
                        let _ = CString::from_raw(item.title);
                    }
                }
            }
        }
    });
}
