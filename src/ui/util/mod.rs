use std::any::Any;

use gtk4::glib::SignalHandlerId;
use gtk4::glib::value::FromValue;
use gtk4::ListItem;
use relm4::binding::Binding;
use relm4::gtk::prelude::*;
use relm4::prelude::*;
use relm4::typed_view::column::RelmColumn;
use relm4::typed_view::OrdFn;

pub trait BindingLabelColumn: 'static{
    /// Item of the model
    type Item: Any;
    /// Value of the column
    type Value: Binding<Target=Self::Target>;
    type Target: for<'a> FromValue<'a>;

    /// Name of the column
    const COLUMN_NAME: &'static str;
    /// Whether to enable the sorting for this column
    const ENABLE_SORT: bool;
    /// Whether to enable resizing for this column
    const ENABLE_RESIZE: bool = false;
    /// Whether to enable automatic expanding for this column
    const ENABLE_EXPAND: bool = false;

    /// Get the value that this column represents.
    fn get_cell_value(item: &Self::Item) -> &Self::Value;
    /// Format the value for presentation in the text cell.
    fn format_cell_value(value: &<Self::Value as Binding>::Target) -> String;
}

pub struct BindingLabelColumnWrapper<C>(C);

impl<C: BindingLabelColumn> RelmColumn for BindingLabelColumnWrapper<C> {
    type Root = gtk::Label;
    type Widgets = Option<SignalHandlerId>;
    type Item = C::Item;
    const COLUMN_NAME: &'static str = C::COLUMN_NAME;
    const ENABLE_RESIZE: bool = C::ENABLE_RESIZE;
    const ENABLE_EXPAND: bool = C::ENABLE_EXPAND;

    fn setup(_list_item: &ListItem) -> (Self::Root, Self::Widgets) {
        (gtk::Label::new(None), None)
    }

    fn bind(item: &mut Self::Item, handler_id: &mut Self::Widgets, root: &mut Self::Root) {
        let value = C::get_cell_value(item);
        let root = root.clone();
        let sighandler = value.connect_notify_local(Some("value"), move |v, _| {
            let value: <C::Value as Binding>::Target = v.get();
            root.set_label(&C::format_cell_value(&value));
        });
        *handler_id = Some(sighandler)
    }

    fn unbind(item: &mut Self::Item, handler_id: &mut Self::Widgets, _root: &mut Self::Root) {
        if let Some(handler_id) = handler_id.take() {
            let value = C::get_cell_value(item);
            value.disconnect(handler_id);
        }
    }

    fn sort_fn() -> OrdFn<Self::Item> {
        None
    }
}