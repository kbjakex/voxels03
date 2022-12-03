// The file where all include_bytes!() invocations shall lie
// because the path for include_bytes! is relative.

macro_rules! include_asset {
    ($path:expr) => {
        include_bytes!(concat!("../../assets/", $path))
    };
}

macro_rules! include_shader {
    ($shader_name:literal) => {{
        #[cfg(not(debug_assertions))]
        {
            include_asset!(concat!("shaders/bin/", $shader_name, ".spv"))
        }
        #[cfg(debug_assertions)]
        {
            include_asset!(concat!("shaders/bin/debug_", $shader_name, ".spv"))
        }
    }};
}

