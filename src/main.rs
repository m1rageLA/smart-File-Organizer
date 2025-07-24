mod errors;
mod history;
mod logger;
mod organizer;
mod rules;
mod ui_cli;
mod ui_gui;

use clap::Parser;
use ui_cli::run_cli;

fn main() -> anyhow::Result<()> {
    let args = ui_cli::CliArgs::parse();
    if args.gui {
        ui_gui::run_gui().map_err(|e| anyhow::anyhow!(e.to_string()))?;
        Ok(())
    } else {
        run_cli()
    }
}
