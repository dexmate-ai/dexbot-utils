fn main() {
    match std::env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("linux") => println!("cargo:rustc-cdylib-link-arg=-Wl,-soname,libdexbot_model.so"),
        Ok("macos") => {
            println!("cargo:rustc-cdylib-link-arg=-Wl,-install_name,@rpath/libdexbot_model.dylib")
        }
        _ => {}
    }
}
