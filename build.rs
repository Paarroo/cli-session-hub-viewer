use lightningcss::{
    bundler::{Bundler, FileProvider},
    stylesheet::{MinifyOptions, ParserOptions, PrinterOptions},
};
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=assets/css/");

    // Create output directory if it doesn't exist
    fs::create_dir_all("assets/dist").expect("Failed to create assets/dist directory");

    // Setup file provider for CSS bundling
    let fs_provider = FileProvider::new();
    let mut bundler = Bundler::new(&fs_provider, None, ParserOptions::default());

    // Bundle CSS starting from main.css
    let mut stylesheet = bundler
        .bundle(Path::new("assets/css/main.css"))
        .expect("Failed to bundle CSS");

    // Minify the bundled stylesheet (in-place)
    stylesheet
        .minify(MinifyOptions::default())
        .expect("Failed to minify CSS");

    // Convert to CSS string with minification
    let css = stylesheet
        .to_css(PrinterOptions {
            minify: true,
            ..Default::default()
        })
        .expect("Failed to generate CSS output");

    // Write bundled CSS to assets/dist/bundle.css
    fs::write("assets/dist/bundle.css", css.code)
        .expect("Failed to write bundle.css");

    println!("CSS bundled successfully: assets/dist/bundle.css");
}
