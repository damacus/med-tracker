use std::fs;

fn main() {
    let source = format!(
        "{}{}",
        leptodon::include_generated::all(),
        leptodon_proc_macros::generate_all_source!()
    );
    fs::write(".tailwind", source).expect("write Leptodon Tailwind classes");
    println!("cargo:rerun-if-changed=src");
}
