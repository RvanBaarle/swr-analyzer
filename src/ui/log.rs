use std::cmp::Ordering;
use std::sync::OnceLock;

use chrono::{DateTime, Local};
use gtk4::{Align, ListItem};
use gtk4::glib::Propagation;
use log::{Level, LevelFilter, Log, Metadata, Record};
use relm4::{Component, ComponentParts, ComponentSender, Sender};
use relm4::prelude::*;
use relm4::prelude::gtk::prelude::*;
use relm4::typed_view::column::{LabelColumn, RelmColumn, TypedColumnView};
use relm4::typed_view::OrdFn;

const TIME_FORMAT: &str = "%H:%M:%S";

static LOGGER: OnceLock<LogSender> = OnceLock::new();

pub struct LogSender(Sender<Input>);

impl Log for LogSender {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        self.0.emit(Input::Log(LogItem {
            date: Local::now(),
            level: record.level(),
            msg: record.args().to_string(),
        }))
    }

    fn flush(&self) {
        // noop
    }
}

#[derive(Debug)]
struct LogItem {
    date: chrono::DateTime<Local>,
    level: Level,
    msg: String,
}

pub struct LogWindow {
    visible: bool,
    list: TypedColumnView<LogItem, gtk::SingleSelection>,
}

#[derive(Debug)]
pub enum Input {
    SetVisible(bool),
    #[doc(hidden)]
    #[allow(private_interfaces)]
    Log(LogItem),
    ToggleVisible,
}

#[relm4::component(pub)]
//noinspection RsSortImplTraitMembers
impl Component for LogWindow {
    type CommandOutput = ();
    type Input = Input;
    type Output = ();
    type Init = ();

    view! {
        gtk::Window {
            set_default_size: (400, 300),
            set_size_request: (400, 300),
            #[watch]
            set_visible: model.visible,

            gtk::ScrolledWindow {
                set_hexpand: true,
                set_vexpand: true,

                #[local_ref]
                column_view -> gtk::ColumnView {
                    set_hexpand: true,
                }
            },

            connect_close_request[sender] => move |_| {
                sender.input(Input::SetVisible(false));
                Propagation::Stop
            }
        }
    }

    fn init(_init: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let mut list = TypedColumnView::new();

        list.append_column::<TimeColumn>();
        list.append_column::<LevelColumn>();
        list.append_column::<MessageColumn>();

        let model = Self {
            visible: true,
            list,
        };

        let column_view = &model.list.view;

        let widgets = view_output!();

        log::set_logger(LOGGER.get_or_init(|| LogSender(sender.input_sender().clone())))
            .expect("logger already set");
        log::set_max_level(LevelFilter::Trace);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            Input::SetVisible(visible) => { self.visible = visible; }
            Input::ToggleVisible => { self.visible = !self.visible; }
            Input::Log(msg) => {
                self.list.append(msg);
            }
        }
    }
}

struct TimeColumn;

struct LevelColumn;

struct MessageColumn;

impl LabelColumn for TimeColumn {
    type Item = LogItem;
    type Value = DateTime<Local>;
    const COLUMN_NAME: &'static str = "time";
    const ENABLE_SORT: bool = true;
    const ENABLE_RESIZE: bool = true;

    fn get_cell_value(item: &Self::Item) -> Self::Value {
        item.date
    }

    fn format_cell_value(value: &Self::Value) -> String {
        value.format(TIME_FORMAT).to_string()
    }
}

impl LabelColumn for LevelColumn {
    type Item = LogItem;
    type Value = Level;
    const COLUMN_NAME: &'static str = "severity";
    const ENABLE_SORT: bool = true;
    const ENABLE_RESIZE: bool = true;

    fn get_cell_value(item: &Self::Item) -> Self::Value {
        item.level
    }
}

impl RelmColumn for MessageColumn {
    type Root = gtk4::Label;
    type Widgets = ();
    type Item = LogItem;
    const COLUMN_NAME: &'static str = "message";
    const ENABLE_RESIZE: bool = true;
    const ENABLE_EXPAND: bool = true;

    fn setup(_list_item: &ListItem) -> (Self::Root, Self::Widgets) {
        let label = gtk4::Label::new(None);
        label.set_halign(Align::Start);
        (label, ())
    }

    fn bind(item: &mut Self::Item, _: &mut Self::Widgets, label: &mut Self::Root) {
        label.set_label(&item.msg);
    }

    fn sort_fn() -> OrdFn<Self::Item> {
        Some(Box::new(|a, b| {
            a.msg.partial_cmp(&b.msg).unwrap_or(Ordering::Equal)
        }))
    }
}