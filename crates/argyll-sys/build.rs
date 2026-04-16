use std::path::PathBuf;

fn main() {
    let argyll_dir: PathBuf = ["external", "argyllcms", "icc"].iter().collect();
    let argyll_dir = std::env::current_dir()
        .unwrap()
        .parent() // crates/
        .unwrap()
        .parent() // workspace root
        .unwrap()
        .join(&argyll_dir);

    cc::Build::new()
        .file(argyll_dir.join("icc.c"))
        .file("argyll_glue.c")
        .include(&argyll_dir)
        .opt_level(2)
        .warnings(false)
        .compile("argyll_icc");
}
