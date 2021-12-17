#[cfg(feature = "sni")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(false)
        // .out_dir("src")
        .build_server(false)
        .compile(&["proto/sni.proto"], &["proto"])?;
    Ok(())
}

#[cfg(not(feature = "sni"))]
fn main() {}
