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
    about = "Автоматически сортирует файлы по подпапкам (CLI/GUI)"
)]
pub struct CliArgs {
    /// Запустить GUI вместо CLI
    #[arg(long)]
    pub gui: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Организовать файлы
    Organize {
        #[arg(short, long)] src: Option<PathBuf>,
        #[arg(short, long)] dst: Option<PathBuf>,
        #[arg(long)] dry_run: bool,
        #[arg(long)] overwrite: bool,
        #[arg(long)] rules: Option<PathBuf>,
    },
    /// Отменить последнее перемещение
    UndoLast {
        #[arg(long, default_value = ".smart_organizer/history.json")]
        history: PathBuf,
    },
    /// Отменить всё
    UndoAll {
        #[arg(long, default_value = ".smart_organizer/history.json")]
        history: PathBuf,
    },
}

pub fn run_cli() -> anyhow::Result<()> {
    let args = CliArgs::parse();

    if args.gui {
        // GUI запускается в main.rs
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
            src, dst, dry_run, overwrite, rules,
        } => {
            let src = src.unwrap_or_else(select_folder_interactive);
            let dst = dst.unwrap_or_else(|| src.clone());

            let history_path = PathBuf::from(".smart_organizer/history.json");
            let log_path     = PathBuf::from(".smart_organizer/organizer.log");

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
            info!("Dry‑run:     {dry_run}");
            info!("Overwrite:   {overwrite}");

            let org = Organizer::new(
                OrganizerConfig { src_dir: src, dst_dir: dst, dry_run, overwrite },
                rule_engine,
                HistoryManager::new(history_path),
            );

            org.organize()?;
        }

        Commands::UndoLast { history } => {
            let org = dummy_organizer(history)?;
            org.undo_last()?;
        }

        Commands::UndoAll { history } => {
            let org = dummy_organizer(history)?;
            org.undo_all()?;
        }
    }
    Ok(())
}

/* ------------------------------------------------------------------ */
/*  !!! Важное изменение: теперь возвращаем конкретный тип с generics  */
/* ------------------------------------------------------------------ */
fn dummy_organizer(history: PathBuf)
    -> anyhow::Result<Organizer<Box<dyn RuleEngine>>>   // ← исправлено
{
    let src = std::env::current_dir()?;
    let dst = src.clone();

    Ok(Organizer::new(
        OrganizerConfig { src_dir: src, dst_dir: dst, dry_run: false, overwrite: false },
        Box::new(ExtensionRuleEngine) as Box<dyn RuleEngine>,
        HistoryManager::new(history),
    ))
}

/* Меню выбора папки */
fn select_folder_interactive() -> PathBuf {
    let theme   = ColorfulTheme::default();
    let current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut dirs = vec![current.clone()];

    if let Ok(read) = std::fs::read_dir(&current) {
        for e in read.flatten() {
            if e.path().is_dir() { dirs.push(e.path()); }
        }
    }

    let items: Vec<String> = dirs.iter().map(|p| p.to_string_lossy().to_string()).collect();
    let selection = Select::with_theme(&theme)
        .with_prompt("Выберите папку для организации")
        .items(&items)
        .default(0)
        .interact()
        .unwrap_or(0);

    dirs[selection].clone()
}
