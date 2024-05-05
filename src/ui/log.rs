use std::sync::mpsc::{channel, Sender};
use std::sync::OnceLock;

use gtk::{glib, SortColumn, SortType, TreeModelSort, TreeStore, TreeView};
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
        if self.log_channel.send((
            record.level(),
            record.args().to_string()
        )).is_err() {
            eprintln!("{} - [{}] {}", chrono::Local::now(), record.level(), record.args())
        }
    }

    fn flush(&self) {}
}

const TIME_FORMAT: &str = "%H:%M:%S";

impl Logger {
    pub fn init(tree_view: TreeView) -> Result<(), log::SetLoggerError> {
        let (send, recv) = channel();
        log::set_logger(LOGGER.get_or_init(||
            Self {
                log_channel: send
            }
        ))?;
        log::set_max_level(LevelFilter::Debug);

        let tree_store = TreeStore::new(&[String::static_type(), String::static_type(), String::static_type(), String::static_type()]);
        let sorted_model = TreeModelSort::new(&tree_store);
        sorted_model.set_sort_column_id(SortColumn::Index(0), SortType::Ascending);

        tree_view.set_model(Some(&sorted_model));

        glib::idle_add_local(clone!(@weak tree_store =>
            @default-return ControlFlow::Break,
            move || {
                if let Ok((lvl, msg)) = recv.try_recv() {
                    let color = match lvl {
                        Level::Error => "red",
                        Level::Warn => "orange",
                        _ => "black",
                    };
                    tree_store.insert_with_values(None, None, &[
                        (0, &chrono::Local::now().format(TIME_FORMAT).to_string()),
                        (1, &lvl.to_string()),
                        (2, &msg),
                        (3, &color)
                    ]);
                }
                ControlFlow::Continue
            }
        ));

        Ok(())
    }
}