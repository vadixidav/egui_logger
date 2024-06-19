#![doc = include_str!("../README.md")]
mod ui;

use std::{cell::Cell, collections::VecDeque, sync::Mutex};

use egui::Color32;
use log::STATIC_MAX_LEVEL;
use ui::{try_mut_log, LoggerUi};

const LOG_MAX_LEN: usize = 10000;

pub struct Builder {
    inner_builder: env_logger::Builder,
    log_to_env_logger: bool,
    log_to_egui_ui: bool,
    ui_level_filter: log::LevelFilter,
}

impl Builder {
    /// Use this to set settings on the [`env_logger::Builder`].
    ///
    /// Note that [`env_logger::Builder::format`] will not work because this crate overrides it.
    pub fn env_logger(
        mut self,
        f: impl FnOnce(&mut env_logger::Builder) -> &mut env_logger::Builder,
    ) -> Self {
        f(&mut self.inner_builder);
        self
    }

    /// Determines whether to log to [`env_logger`] or not.
    ///
    /// This wil output to stdout or stderr based on your settings.
    ///
    /// Default: `true`
    pub fn log_to_env_logger(self, log_to_env_logger: bool) -> Self {
        Self {
            log_to_env_logger,
            ..self
        }
    }

    /// Determines whether to log to the [`egui`] widget.
    ///
    /// Default: `true`
    pub fn log_to_egui_ui(self, log_to_egui_ui: bool) -> Self {
        Self {
            log_to_egui_ui,
            ..self
        }
    }

    /// Limits logs specifically going to the UI, but not which are logged with [`env_logger`].
    ///
    /// This is useful since there is a limit on how many logs are recorded for the UI.
    ///
    /// If you don't call this, only [`env_logger`]'s filter will be used.
    pub fn ui_level_filter(self, ui_level_filter: log::LevelFilter) -> Self {
        Self {
            ui_level_filter,
            ..self
        }
    }

    /// Builds the logger.
    pub fn build(self) -> Logger {
        let Self {
            mut inner_builder,
            log_to_env_logger,
            log_to_egui_ui,
            ui_level_filter,
        } = self;
        Logger {
            inner_logger: inner_builder.build(),
            log_to_env_logger,
            log_to_egui_ui,
            ui_level_filter,
        }
    }

    /// Builds and sets the logger as the global logger.
    pub fn init(self) -> Result<(), log::SetLoggerError> {
        log::set_boxed_logger(Box::new(self.build()))
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            inner_builder: Default::default(),
            log_to_env_logger: true,
            log_to_egui_ui: true,
            ui_level_filter: STATIC_MAX_LEVEL,
        }
    }
}

/// The egui logger.
pub struct Logger {
    inner_logger: env_logger::Logger,
    log_to_env_logger: bool,
    log_to_egui_ui: bool,
    ui_level_filter: log::LevelFilter,
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.inner_logger.enabled(metadata)
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            if self.log_to_egui_ui && record.level() <= self.ui_level_filter {
                thread_local! {
                    pub static LOG_VEC: Cell<Vec<u8>> = Cell::new(Vec::new());
                }
                let mut log_vec = LOG_VEC.take();
                if self.log_to_env_logger {
                    self.inner_logger.dual_log(&mut log_vec, record);
                } else {
                    self.inner_logger.write_log(&mut log_vec, record);
                }
                let log_str = String::from_utf8_lossy(&log_vec).into_owned();
                try_mut_log(|logs| {
                    logs.push_front((record.level(), log_str));
                    logs.truncate(LOG_MAX_LEN);
                });
                LOG_VEC.set(log_vec);
            } else if self.log_to_env_logger {
                self.inner_logger.log(record);
            }
        }
    }

    fn flush(&self) {
        self.inner_logger.flush();
    }
}

pub(crate) type GlobalLog = VecDeque<(log::Level, String)>;

static LOG: Mutex<GlobalLog> = Mutex::new(VecDeque::new());

fn log_ui() -> &'static Mutex<LoggerUi> {
    static LOGGER_UI: std::sync::OnceLock<Mutex<LoggerUi>> = std::sync::OnceLock::new();
    LOGGER_UI.get_or_init(Default::default)
}

/// Render the logger UI.
pub fn ui(ui: &mut egui::Ui) {
    ui_filter(ui, log::LevelFilter::Info);
}

/// Render the logger UI with a log level filter.
pub fn ui_filter(ui: &mut egui::Ui, level_filter: log::LevelFilter) {
    if let Ok(ref mut logger_ui) = log_ui().lock() {
        logger_ui.ui(ui, level_filter);
    } else {
        ui.colored_label(Color32::RED, "Something went wrong loading the log");
    }
}

/// Clear the logs.
pub fn clear() {
    try_mut_log(VecDeque::clear);
}

/**
This returns the Log builder with default values.
[Read more](`crate::Builder`)

Example:
```rust
use log::LevelFilter;
fn main() -> {
    // initialize the logger.
    // You have to open the ui later within your egui context logic.
    // You should call this very early in the program.
    egui_logger::builder()
        .max_level(LevelFilter::Info) // defaults to Debug
        .init()
        .unwrap();

    // ...
}
```
*/
pub fn builder() -> Builder {
    Default::default()
}
