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
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            let backtrace = gen_backtrace_string();
            error!("{message}\nBacktrace:\n{backtrace}");
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => warn!("{message}"),
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => debug!("{message}"),
        _ => info!("{message}"),
    }

    vk::FALSE
}

fn gen_backtrace_string() -> String {
    // The code might not be fast or pretty, but the output is useful
    use std::fmt::Write;
    let trace = std::backtrace::Backtrace::force_capture();
    // For whatever reason, it prints the 100% complete backtrace,
    // which is not useful; really only the entries from this crate
    // are of interest
    let mut trace_string = String::new();
    _ = write!(&mut trace_string, "{trace}");

    // First: the backtrace will uselessly start from this module,
    // followed by a bunch of <unknown>s because it went through Vulkan.
    // Keep cutting off stuff until no <unknown>s are found
    while let Some(i) = trace_string.find("<unknown>") {
        trace_string = (&trace_string[i + "<unknown>".len()..]).into();
    }

    // Remove the remains of the first line...
    for i in 0..trace_string.len()-1 {
        if trace_string.as_bytes()[i] == b'\n' {
            trace_string = (&trace_string[i+1..]).into();
            break;
        }
    }

    trace_string = "...\n".to_owned() + &trace_string;

    // Cut off at winit, since that's certainly far away enough
    if let Some(mut i) = trace_string.find("winit") {
        let substr = &trace_string[..i];
        while i > 1 && substr.as_bytes()[i-1] != b'\n' {
            i -= 1;
        }
        trace_string = (&trace_string[..i]).to_owned() + "...";
    }

    trace_string
}