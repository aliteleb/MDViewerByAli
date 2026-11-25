fn main() {
    println!("cargo:rerun-if-changed=assets/app/app.rc");
    println!("cargo:rerun-if-changed=assets/app/icon.ico");
    println!("cargo:rerun-if-changed=assets/app/md-doc.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile("assets/app/app.rc", embed_resource::NONE)
            .manifest_optional()
            .unwrap();
    }
}
