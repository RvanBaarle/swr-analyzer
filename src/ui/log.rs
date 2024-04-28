use std::sync::{Mutex, OnceLock};
use std::sync::mpsc::{channel, Sender};

use gtk::{glib, TreeStore, TreeView};
use gtk::glib::{clone, ControlFlow};
use gtk::prelude::*;
use log::{Level, LevelFilter, Metadata, Record};

static LOGGER: OnceLock<Logger> = OnceLock::new();

pub struct Logger {
    log_channel: Sender<(Level, String)>,
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() >= Level::Debug
    }

    fn log(&self, record: &Record) {
        if let Err(e) = self.log_channel.send((
            record.level(),
            record.args().to_string()
        )) {
            eprintln!("{} - [{}] {}", chrono::Local::now(), record.level(), record.args())
        }
    }

    fn flush(&self) {}
}

const TIME_FORMAT: &'static str = "%H:%M:%S";

impl Logger {
    pub fn init(tree_view: TreeView) -> Result<(), log::SetLoggerError> {
        let (send, recv) = channel();
        log::set_logger(LOGGER.get_or_init(||
            Self {
                log_channel: send
            }
        ))?;
        log::set_max_level(LevelFilter::Debug);

        let tree_store = TreeStore::new(&[String::static_type(), String::static_type(), String::static_type()]);
        
        tree_view.set_model(Some(&tree_store));

        glib::idle_add_local(clone!(@weak tree_store =>
            @default-return ControlFlow::Break,
            move || {
            if let Ok((lvl, msg)) = recv.try_recv() {
                tree_store.insert_with_values(None, None, &[
                    (0, &chrono::Local::now().format(TIME_FORMAT).to_string()),
                    (1, &lvl.to_string()),
                    (2, &msg)
                ]);
            }
            ControlFlow::Continue
        }));

        Ok(())
    }
}