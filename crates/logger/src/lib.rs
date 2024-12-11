use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

static LOG_LEVEL: AtomicUsize = AtomicUsize::new(0);
static PROGRESS_BAR: Lazy<Arc<Mutex<Option<ProgressBar>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

/// Logger is a static class that provides logging functionality.
///
/// # Examples
///
/// ```
/// use spinne_logger::Logger;
/// Logger::info("Hello, world!");
/// Logger::warn("This is a warning!");
/// Logger::error("This is an error!");
/// Logger::set_level(1);
/// Logger::debug("This is a debug message with level 1!", 1);
/// Logger::set_level(2);
/// Logger::debug("This is a debug message with level 2!", 2);
/// Logger::loading("Loading...");
/// Logger::done_loading();
/// ```
pub struct Logger;

impl Logger {
    pub fn set_level(level: u8) {
        LOG_LEVEL.store(level as usize, Ordering::Relaxed);
    }

    pub fn info(msg: &str) {
        // Clear progress bar if it exists, print message, then restore progress bar
        if let Some(pb) = PROGRESS_BAR.lock().unwrap().as_ref() {
            pb.suspend(|| {
                println!("{}", msg.blue());
            });
        } else {
            println!("{}", msg.blue());
        }
    }

    pub fn warn(msg: &str) {
        if let Some(pb) = PROGRESS_BAR.lock().unwrap().as_ref() {
            pb.suspend(|| {
                println!("{}", msg.yellow());
            });
        } else {
            println!("{}", msg.yellow());
        }
    }

    pub fn error(msg: &str) {
        if let Some(pb) = PROGRESS_BAR.lock().unwrap().as_ref() {
            pb.suspend(|| {
                eprintln!("{}", msg.red());
            });
        } else {
            eprintln!("{}", msg.red());
        }
    }

    pub fn debug(msg: &str, level: usize) {
        if LOG_LEVEL.load(Ordering::Relaxed) >= level {
            if let Some(pb) = PROGRESS_BAR.lock().unwrap().as_ref() {
                pb.suspend(|| {
                    println!("{}", msg.magenta());
                });
            } else {
                println!("{}", msg.magenta());
            }
        }
    }

    pub fn loading(msg: &str) {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈")
                .template("{spinner:.blue} {msg}")
                .unwrap(),
        );
        pb.set_message(msg.to_string());
        pb.enable_steady_tick(Duration::from_millis(80));

        *PROGRESS_BAR.lock().unwrap() = Some(pb);
    }

    pub fn done_loading() {
        if let Some(pb) = PROGRESS_BAR.lock().unwrap().take() {
            pb.finish_and_clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_levels() {
        Logger::set_level(2);

        // Should not print
        Logger::debug("Level 3 message", 3);

        // Should print
        Logger::debug("Level 2 message", 2);
        Logger::debug("Level 1 message", 1);
    }

    #[test]
    fn test_loading_indicator() {
        Logger::loading("Processing files...");
        std::thread::sleep(std::time::Duration::from_millis(500));
        Logger::info("Some info while loading");
        std::thread::sleep(std::time::Duration::from_millis(500));
        Logger::done_loading();
    }
}
