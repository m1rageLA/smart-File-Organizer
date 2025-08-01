// src/ui_gui.rs

use crate::{
    history::HistoryManager,
    organizer::{Organizer, OrganizerConfig},
    rules::{ExtensionRuleEngine, RuleEngine},
};
use crossbeam_channel::{bounded, Receiver};
use eframe::{App, Frame};
use egui::{Context, RichText};
use log::{error, info};
use parking_lot::Mutex;
use rfd::FileDialog;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

pub fn run_gui() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Smart File Organizer",
        native_options,
        Box::new(|_cc| Box::new(GuiApp::default())),
    )
}

struct GuiApp {
    src: Option<PathBuf>,
    dst: Option<PathBuf>,
    running: bool,
    progress: Arc<AtomicBool>,
    last_error: Arc<Mutex<Option<String>>>,
    receiver: Option<Receiver<()>>,
    overwrite: bool,
    dry_run: bool,
}

impl Default for GuiApp {
    fn default() -> Self {
        Self {
            src: None,
            dst: None,
            running: false,
            progress: Arc::new(AtomicBool::new(false)),
            last_error: Arc::new(Mutex::new(None)),
            receiver: None,
            overwrite: false,
            dry_run: false,
        }
    }
}

impl App for GuiApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Smart File Organizer (Rust + egui)");

            ui.horizontal(|ui| {
                if ui.button("Select source…").clicked() {
                    if let Some(path) = FileDialog::new().pick_folder() {
                        self.src = Some(path);
                    }
                }
                let src_label = self
                    .src
                    .as_ref()
                    .map_or("—".to_string(), |p| p.to_string_lossy().to_string());
                ui.label(src_label);
            });

            ui.horizontal(|ui| {
                if ui.button("Select destination…").clicked() {
                    if let Some(path) = FileDialog::new().pick_folder() {
                        self.dst = Some(path);
                    }
                }
                let dst_label = self
                    .dst
                    .as_ref()
                    .map_or("—".to_string(), |p| p.to_string_lossy().to_string());
                ui.label(dst_label);
            });

            ui.checkbox(&mut self.dry_run, "Dry-run mode");
            ui.checkbox(&mut self.overwrite, "Overwrite conflicting files");

            ui.separator();

            if !self.running {
                if ui.button("Start").clicked() {
                    if let Some(src) = self.src.clone() {
                        let dst = self.dst.clone().unwrap_or_else(|| src.clone());
                        let dry_run = self.dry_run;
                        let overwrite = self.overwrite;

                        let (tx, rx) = bounded(1);
                        self.receiver = Some(rx);

                        let progress = self.progress.clone();
                        let last_error = self.last_error.clone();
                        self.running = true;

                        thread::spawn(move || {
                            progress.store(true, Ordering::Relaxed);

                            let history_path = PathBuf::from(".smart_organizer/history.json");
                            std::fs::create_dir_all(".smart_organizer").ok();

                            let organizer = Organizer::new(
                                OrganizerConfig {
                                    src_dir: src,
                                    dst_dir: dst,
                                    dry_run,
                                    overwrite,
                                },
                                ExtensionRuleEngine,
                                HistoryManager::new(history_path),
                            );

                            if let Err(e) = organizer.organize() {
                                error!("Organize error: {}", e);
                                *last_error.lock() = Some(e.to_string());
                            }

                            progress.store(false, Ordering::Relaxed);
                            let _ = tx.send(());
                        });
                    }
                }
            } else if ui.button("Cancel").clicked() {
                self.progress.store(false, Ordering::Relaxed);
            }

            if let Some(rx) = &self.receiver {
                if rx.try_recv().is_ok() {
                    self.running = false;
                    self.receiver = None;
                }
            }

            if self.running && self.progress.load(Ordering::Relaxed) {
                ui.label(RichText::new("Working…").italics());
                ctx.request_repaint_after(Duration::from_millis(200));
            } else if self.running {
                ui.label(RichText::new("Done").strong());
            }

            if let Some(err) = self.last_error.lock().clone() {
                ui.colored_label(egui::Color32::RED, format!("Last error: {}", err));
            }
        });
    }
}
