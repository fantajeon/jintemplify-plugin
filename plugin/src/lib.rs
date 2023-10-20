use serde::Serialize;

#[macro_export]
macro_rules! guest_plugin {
    () => {
        #[no_mangle]
        extern "C" {
            fn print_log_from_wasm(ptr: *const u8, len: usize);
        }

        #[no_mangle]
        extern "C" fn guest_free(ptr: *mut jintemplify_plugin::ReturnValues) {
            free_return_values(ptr);
        }

        pub fn free_return_values(ptr: *mut jintemplify_plugin::ReturnValues) {
            let boxed = unsafe { Box::from_raw(ptr) };
            let _ = unsafe {
                Vec::from_raw_parts(boxed.ptr as *mut u8, boxed.len as usize, boxed.cap as usize)
            };
        }

        pub fn send_log(message: &str) {
            unsafe {
                print_log_from_wasm(message.as_ptr(), message.len());
            }
        }
    };
}

#[macro_export]
macro_rules! host_plugin {
    () => {};
}

#[allow(dead_code)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct InputWrapper {
    pub params: Vec<serde_json::Value>,
}

#[allow(dead_code)]
#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct OutputWrapper {
    pub result: serde_json::Value,
}

#[allow(dead_code)]
#[repr(C)]
pub struct ReturnValues {
    pub ptr: u32,
    pub len: u32,
    pub cap: u32,
}

#[allow(dead_code)]
#[derive(Serialize)]
pub struct ErrorValue {
    pub reason: String,
}

pub fn serialize_to_return_values<T: serde::Serialize>(data: &T) -> *mut ReturnValues {
    let output_json = serde_json::to_string(data).expect("Failed to serialize data to JSON");
    let output_bytes = output_json.into_bytes();
    let output_len = output_bytes.len();
    let output_ptr = output_bytes.as_ptr();
    let output_cap = output_bytes.capacity();
    std::mem::forget(output_bytes);

    let return_values = Box::new(ReturnValues {
        ptr: output_ptr as u32,
        len: output_len as u32,
        cap: output_cap as u32,
    });
    Box::into_raw(return_values)
}

pub fn convert_value<T: serde::de::DeserializeOwned>(
    value: &serde_json::Value,
    index: usize,
) -> Result<T, String> {
    serde_json::from_value(value.clone())
        .map_err(|err| format!("Error at index {}: {}", index, err))
}
