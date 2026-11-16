// Embeds the icon, Windows version information and the manifest (runs as a normal user, Windows 10+, DPI aware).
fn main() {
    println!("cargo:rerun-if-changed=res/app.rc");
    println!("cargo:rerun-if-changed=res/app.manifest");
    println!("cargo:rerun-if-changed=res/app.ico");
    embed_resource::compile("res/app.rc", embed_resource::NONE).manifest_required().unwrap();
}
