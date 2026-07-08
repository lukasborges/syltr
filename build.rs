//! Compiles the bundled service icons into a GResource embedded in the binary.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let target = Path::new(&out_dir).join("syltr.gresource");
    let xml = manifest.join("data/syltr.gresource.xml");
    let source_dir = manifest.join("data/service-icons");

    let status = Command::new("glib-compile-resources")
        .arg("--sourcedir")
        .arg(&source_dir)
        .arg("--target")
        .arg(&target)
        .arg(&xml)
        .status()
        .expect("glib-compile-resources not found (install glib2)");
    assert!(status.success(), "glib-compile-resources failed");

    println!("cargo:rerun-if-changed={}", xml.display());
    println!("cargo:rerun-if-changed={}", source_dir.display());
}
