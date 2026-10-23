use std::{
    env, fs,
    io::{self, Write},
    process,
};

fn main() -> io::Result<()> {
    let arguments: Vec<_> = env::args().skip(1).collect();
    if arguments.get(1).map(String::as_str) == Some("fail") {
        process::exit(37);
    }
    let path = arguments
        .get(2)
        .ok_or_else(|| io::Error::other("missing probe path"))?;
    let mut report = fs::File::create(path)?;
    report.write_all(arguments.join("\0").as_bytes())?;
    write!(
        report,
        "\0{}\0{}",
        env::var("LOCO_ENV").unwrap_or_default(),
        env::var("PATH").unwrap_or_default()
    )?;
    println!("synthetic scheduler stdout");
    eprintln!("synthetic scheduler stderr");
    Ok(())
}
