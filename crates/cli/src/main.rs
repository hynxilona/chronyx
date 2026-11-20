mod cfg;
mod cmd;
mod net;

use clap::Parser;

fn main() -> anyhow::Result<()> {
    let cli = cmd::Cli::parse();
    cmd::run(cli.cmd, cli.url)
}
