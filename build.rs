//! Puts the icon and the version information of the program into the Windows executable.
//! On every other target nothing happens, and a missing icon or resource compiler only gives a warning.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=images/brand/app.ico");
    #[cfg(windows)]
    embed();
}

#[cfg(windows)]
fn embed() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let icon = "images/brand/app.ico";
    if !std::path::Path::new(icon).is_file() {
        println!("cargo:warning=icon {icon} not found, the executable gets no icon");
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon(icon)
        .set("ProductName", "EH3D-PHOENIX-LASER")
        .set("FileDescription", "Laser engraving from an image")
        .set("CompanyName", "JuanenRac (Electro Hobby 3D)")
        .set(
            "LegalCopyright",
            "Copyright (C) JuanenRac (Electro Hobby 3D)",
        );
    if let Err(e) = res.compile() {
        println!("cargo:warning=could not embed the icon and version information: {e}");
    }
}
