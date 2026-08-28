//! Build script — Gate 6/7 DeckLink SDK bindgen pipeline.
//!
//! Two modes:
//! - `bmd` feature ON + `DECKLINK_SDK_INCLUDE` set  -> run bindgen on DeckLinkAPI.h,
//!   emit real FFI bindings to `OUT_DIR/bindings.rs` (consumed by `src/decklink.rs`).
//! - otherwise                                          -> emit an empty `bindings.rs` stub
//!   (`src/decklink.rs` takes the stub/error path).
//!
//! The `bmd` feature is OFF by default so CI / non-BMD builds compile without the
//! proprietary SDK header or libclang. Real enumeration is validated on BMD (runc + Option B).

fn main() {
    #[cfg(feature = "bmd-provider")]
    gen_real();

    #[cfg(not(feature = "bmd-provider"))]
    gen_stub();
}

#[cfg(feature = "bmd-provider")]
fn gen_real() {
    let include = std::env::var("DECKLINK_SDK_INCLUDE").expect(
        "DECKLINK_SDK_INCLUDE must point at the DeckLink SDK Linux/include dir (Gate 6/7)",
    );
    let header = format!("{include}/DeckLinkAPI.h");

    let bindings = bindgen::Builder::default()
        .header(header)
        .clang_arg("-std=c++11")
        // DeckLinkAPI.h is a C++ COM header; bindgen must parse it as C++.
        .clang_arg("-x")
        .clang_arg("c++")
        // Keep the generated vtable/struct layout usable from safe-ish Rust.
        .derive_debug(false)
        .generate()
        .expect("Unable to generate DeckLink bindings (check DECKLINK_SDK_INCLUDE / libclang)");

    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out.join("bindings.rs"))
        .expect("Couldn't write bindings.rs");
}

#[cfg(not(feature = "bmd-provider"))]
fn gen_stub() {
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    std::fs::write(out.join("bindings.rs"), "// stub: bmd feature disabled at build time\n")
        .expect("Couldn't write stub bindings.rs");
}
