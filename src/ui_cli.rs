// src/ui_cli.rs

use crate::{
    history::HistoryManager,
    logger::setup_logging,
    organizer::{Organizer, OrganizerConfig},
    rules::{CustomRuleEngine, ExtensionRuleEngine, RuleEngine},
};
use clap::{Parser, Subcommand};
use dialoguer::{theme::ColorfulTheme, Select};
use log::info;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "Smart File Organizer",
    version,
    about = "Automatically sorts files into subfolders (CLI/GUI)"
)]
pub struct CliArgs {
    /// Launch GUI instead of CLI
    #[arg(long)]
    pub gui: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Organize files
    Organize {
        #[arg(short, long)] src: Option<PathBuf>,
        #[arg(short, long)] dst: Option<PathBuf>,
        #[arg(long)] dry_run: bool,
        #[arg(long)] overwrite: bool,
        #[arg(long)] rules: Option<PathBuf>,
    },
    /// Undo last move
    UndoLast {
        #[arg(long, default_value = ".smart_organizer/history.json")]
        history: PathBuf,
    },
    /// Undo all moves
    UndoAll {
        #[arg(long, default_value = ".smart_organizer/history.json")]
        history: PathBuf,
    },
}

pub fn run_cli() -> anyhow::Result<()> {
    let args = CliArgs::parse();

    if args.gui {
        // GUI is launched from main.rs
        return Ok(());
    }

    match args.command.unwrap_or(Commands::Organize {
        src: None,
        dst: None,
        dry_run: false,
        overwrite: false,
        rules: None,
    }) {
        Commands::Organize {
            src,
            dst,
            dry_run,
            overwrite,
            rules,
        } => {
            let src = src.unwrap_or_else(select_folder_interactive);
            let dst = dst.unwrap_or_else(|| src.clone());

            let history_path = PathBuf::from(".smart_organizer/history.json");
            let log_path = PathBuf::from(".smart_organizer/organizer.log");

            std::fs::create_dir_all(".smart_organizer")?;
            setup_logging(log_path)?;

            let rule_engine: Box<dyn RuleEngine> = if let Some(rules_json) = rules {
                let text = std::fs::read_to_string(rules_json)?;
                Box::new(serde_json::from_str::<CustomRuleEngine>(&text)?) as _
            } else {
                Box::new(ExtensionRuleEngine) as _
            };

            info!("Source:      {:?}", src);
            info!("Destination: {:?}", dst);
            info!("Dry‑run:     {}", dry_run);
            info!("Overwrite:   {}", overwrite);

            let organizer = Organizer::new(
                OrganizerConfig {
                    src_dir: src,
                    dst_dir: dst,
                    dry_run,
                    overwrite,
                },
                rule_engine,
                HistoryManager::new(history_path),
            );

            organizer.organize()?;
        }

        Commands::UndoLast { history } => {
            let organizer = dummy_organizer(history)?;
            organizer.undo_last()?;
        }

        Commands::UndoAll { history } => {
            let organizer = dummy_organizer(history)?;
            organizer.undo_all()?;
        }
    }

    Ok(())
}

/// Returns an Organizer with default settings for undo commands
fn dummy_organizer(
    history: PathBuf,
) -> anyhow::Result<Organizer<Box<dyn RuleEngine>>> {
    let src = std::env::current_dir()?;
    let dst = src.clone();

    Ok(Organizer::new(
        OrganizerConfig {
            src_dir: src,
            dst_dir: dst,
            dry_run: false,
            overwrite: false,
        },
        Box::new(ExtensionRuleEngine) as Box<dyn RuleEngine>,
        HistoryManager::new(history),
    ))
}

/// Simple interactive folder selection menu
fn select_folder_interactive() -> PathBuf {
    let theme = ColorfulTheme::default();
    let current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut dirs = vec![current.clone()];

    if let Ok(read_dir) = std::fs::read_dir(&current) {
        for entry in read_dir.flatten() {
            if entry.path().is_dir() {
                dirs.push(entry.path());
            }
        }
    }

    let items: Vec<String> = dirs
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    let selection = Select::with_theme(&theme)
        .with_prompt("Select a folder to organize")
        .items(&items)
        .default(0)
        .interact()
        .unwrap_or(0);

    dirs[selection].clone()
}
