
extern crate bindgen;

fn sdk_path() -> Result<String, std::io::Error> {
    use std::process::Command;

    let output = Command::new("xcrun")
        .args(&["--sdk", "iphoneos", "--show-sdk-path"])
        .output()?
        .stdout;
    let prefix_str = std::str::from_utf8(&output).expect("invalid output from `xcrun`");
    Ok(prefix_str.trim_end().to_string())
}

fn build(sdk_path: Option<&str>, target: &str) {
    // Generate one large set of bindings for all frameworks.
    //
    // We do this rather than generating a module per framework as some frameworks depend on other
    // frameworks and in turn share types. To ensure all types are compatible across each
    // framework, we feed all headers to bindgen at once.
    //
    // Only link to each framework and include their headers if their features are enabled and they
    // are available on the target os.

    use std::env;
    use std::path::PathBuf;

    let mut headers : Vec<&str> = vec![];

    println!("cargo:rustc-link-lib=framework=UIKit");
    headers.push("UIKit/UIKit.h");

    println!("cargo:rerun-if-env-changed=BINDGEN_EXTRA_CLANG_ARGS");
    // Get the cargo out directory.
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("env variable OUT_DIR not found"));

    // Begin building the bindgen params.
    let mut builder = bindgen::Builder::default();

    builder = builder.clang_args(&["-x", "objective-c", "-fblocks"]);
    builder = builder.objc_extern_crate(true);
    builder = builder.block_extern_crate(true);
    builder = builder.generate_block(true);
    builder = builder.rustfmt_bindings(true);
    // See https://github.com/rust-lang/rust-bindgen/issues/1211
    // Technically according to the llvm mailing list, the argument to clang here should be
    // -arch arm64 but it looks cleaner to just change the target.
    let target = if target == "aarch64-apple-ios" {
        "arm64-apple-ios"
    } else {
        target
    };

    builder = builder.clang_args(&[&format!("--target={}", target)]);

    if let Some(sdk_path) = sdk_path {
        builder = builder.clang_args(
            &[
            "-isysroot", sdk_path,
            ]
            );
    }
    if target.contains("apple-ios") {
        builder = builder.clang_args(
            &[
            "-x", "objective-c",
            "-fblocks",
            ]
        );
        builder = builder.objc_extern_crate(true);
        builder = builder.block_extern_crate(true);
        builder = builder.generate_block(true);
        builder = builder.rustfmt_bindings(true);

        // time.h as has a variable called timezone that conflicts with some of the objective-c
        // calls from NSCalendar.h in the Foundation framework. This removes that one variable.
        builder = builder.blacklist_item("timezone");
        // https://github.com/rust-lang/rust-bindgen/issues/1705
        builder = builder.blacklist_item("UIStepper");
        //builder = builder.blacklist_item("dividerImageForLeftSegmentState:rightSegmentState:");
    }

    let meta_header: Vec<_> = headers
        .iter()
        .map(|h| format!("#include <{}>\n", h))
        .collect();

    //builder = builder.header_contents("./test.h", &meta_header.concat());
    builder = builder.header_contents("UIKit.h", &meta_header.concat());

    // Generate the bindings.
    builder = builder
        .trust_clang_mangling(false)
        .derive_default(true);

    let bindings = builder.generate().expect("unable to generate bindings");

    // Write them to the crate root.
    bindings
        .write_to_file(out_dir.join("uikit.rs"))
        .expect("could not write bindings");
}

fn main() {

    let target = std::env::var("TARGET").unwrap();
    if !target.contains("apple-ios") {
        panic!("coreaudio-sys requires macos or ios target");
    }

    let directory = sdk_path().ok();
    build(directory.as_ref().map(String::as_ref), &target);
}