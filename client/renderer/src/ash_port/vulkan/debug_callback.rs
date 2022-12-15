use std::{borrow::Cow, ffi::CStr};

use ash::{extensions::ext::DebugUtils, vk, Entry, Instance};
use log::{info, debug, warn, error};

pub struct DebugMessageHandler {
    pub debug_utils: DebugUtils,
    pub debug_callback: vk::DebugUtilsMessengerEXT,
}

impl DebugMessageHandler {
    pub fn new(entry: &Entry, instance: &Instance) -> Self {
        let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(debug_callback));

        let debug_utils = DebugUtils::new(&entry, &instance);
        let debug_callback =
            unsafe { debug_utils.create_debug_utils_messenger(&debug_info, None) }.unwrap();

        Self {
            debug_utils,
            debug_callback,
        }
    }
}
unsafe extern "system" fn debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;
    let message_id_number: i32 = callback_data.message_id_number as i32;

    let message_id_name = if callback_data.p_message_id_name.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
    };

    // These appear to be spam rather than anything useful
    if message_id_name.contains("Loader") {
        return vk::FALSE
    }

    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };


    let message = format!("[{message_type:?} : {message_id_name} ({message_id_number})]\n{message}\n");

    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => error!("{message}"),
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => warn!("{message}"),
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => debug!("{message}"),
        _ => info!("{message}"),
    }

    vk::FALSE
}
