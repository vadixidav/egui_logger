use egui::Color32;
use log::LevelFilter;

use crate::{GlobalLog, LOG};

pub(crate) fn try_mut_log<F, T>(f: F) -> Option<T>
where
    F: FnOnce(&mut GlobalLog) -> T,
{
    match LOG.lock() {
        Ok(ref mut global_log) => Some((f)(global_log)),
        Err(_) => None,
    }
}

fn try_get_log<F, T>(f: F) -> Option<T>
where
    F: FnOnce(&GlobalLog) -> T,
{
    match LOG.lock() {
        Ok(ref global_log) => Some((f)(global_log)),
        Err(_) => None,
    }
}

/// Runs the given function on all the logs at or below the level filter.
///
/// Returns the number of logs processed.
fn log_filter_process(level_filter: LevelFilter, mut f: impl FnMut(log::Level, &str)) -> usize {
    let mut logs_processed: usize = 0;
    try_get_log(|logs| {
        for (level, line) in logs.iter().filter(|&&(level, _)| level <= level_filter) {
            logs_processed += 1;
            f(*level, line)
        }
    });
    logs_processed
}

pub(crate) struct LoggerUi {}

impl Default for LoggerUi {
    fn default() -> Self {
        Self {}
    }
}

impl LoggerUi {
    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, level_filter: log::LevelFilter) {
        ui.separator();

        let mut logs_displayed: usize = 0;

        egui::ScrollArea::vertical()
            .auto_shrink([false, true])
            .max_height(ui.available_height() - 30.0)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                logs_displayed = log_filter_process(level_filter, |level, line| {
                    let string_format = format!("{}", line);

                    match level {
                        log::Level::Warn => ui.colored_label(Color32::YELLOW, string_format),
                        log::Level::Error => ui.colored_label(Color32::RED, string_format),
                        _ => ui.label(string_format),
                    };
                });
            });
        ui.separator();

        ui.horizontal(|ui| {
            ui.label(format!(
                "Log size: {}",
                try_get_log(|logs| logs.len()).unwrap_or_default()
            ));
            ui.label(format!("Displayed: {}", logs_displayed));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Copy Logs").clicked() {
                    ui.output_mut(|o| {
                        try_get_log(|logs| {
                            let mut out_string = String::new();
                            logs.iter().for_each(|(_, string)| {
                                out_string.push_str(string);
                                out_string.push_str(" \n");
                            });
                            o.copied_text = out_string;
                        });
                    });
                }
            });
        });
    }
}
