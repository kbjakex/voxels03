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

pub mod shaders {
    // The vertex shader for all full cubes with textures, i.e the main
    // vertex shader that renders the majority of the world
    pub const TEXTURED_FULL_CUBE_VERT : &[u8] = include_shader!("textured_full_cube.vert");
    
    // The fragment shader for all textured + lit geometry
    pub const TEXTURED_LIT_FRAG : &[u8] = include_shader!("textured_lit.frag");
}