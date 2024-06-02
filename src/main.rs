use std::{env, io};
use std::fs::File;
use std::io::{ErrorKind, Write};
use clap::Parser;
use relm4::RelmApp;

use ui::App;

mod protocol;
mod ui;

#[derive(Parser)]
#[command(version, about)]
struct Args {
    #[arg(short, long)]
    udev: bool,
    #[arg(long)]
    no_elevate: bool,
}

fn main() {
    let args: Args = Args::parse();

    if args.udev {
        if let Err(e) = try_install_udev(!args.no_elevate) {
            eprintln!("Installing rules failed: {}", e);
        }
        return;
    }
    
    let app = RelmApp::new("nl.vbaarle.ruben.swranalyzer");
    app.run::<App>(());
}

fn try_install_udev(elevate: bool) -> io::Result<()> {
    if let Err(e) = install_udev() {
        if e.kind() == ErrorKind::PermissionDenied  && elevate {
            install_udev_elevated()?;
        } else {
            return Err(e);
        }
    }
    Ok(())
}

fn install_udev() -> io::Result<()> {
    let rules = include_str!("../udev/99-swr-analyzer.rules");
    let mut f = File::create_new("/etc/udev/rules.d/99-swr-analyzer.rules")?;
    f.write_all(rules.as_bytes())?;
    Ok(())
}

fn install_udev_elevated() -> io::Result<()> {
    let executable = std::env::args().next().unwrap();
    let result = std::process::Command::new("pkexec")
        .args(&[&executable, "-u", "--no-elevate"])
        .status()?;
    println!("exit with: {}", result);
    Ok(())
}