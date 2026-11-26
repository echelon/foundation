#[cfg(unix)]
mod backup;
#[cfg(unix)]
mod connector;
#[cfg(unix)]
mod dev;
#[cfg(unix)]
mod mapping;
#[cfg(unix)]
mod postgres;
#[cfg(unix)]
mod state;

#[cfg(unix)]
fn main() -> anyhow::Result<()> {
    dev::main()
}

#[cfg(not(unix))]
fn main() {
    eprintln!("tokencraft dev requires macOS or Linux with local PostgreSQL tools.");
    std::process::exit(1);
}
