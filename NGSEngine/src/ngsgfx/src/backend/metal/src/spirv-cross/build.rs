extern crate gcc;

fn main() {
    gcc::Config::new()
        .cpp(true)
        .flag("-std=c++11") // TODO: support MSVC!
        .file("libspirvcross/spirv_cfg.cpp")
        .file("libspirvcross/spirv_cross.cpp")
        .file("libspirvcross/spirv_glsl.cpp")
        .file("libspirvcross/spirv_msl.cpp")
        .file("binding/spirv2msl.cpp")
        .compile("libspirvcross.a");
}
