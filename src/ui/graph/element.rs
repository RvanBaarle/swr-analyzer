use gtk4::{GestureClick, ListItem, MultiSelection};
use gtk4::glib::{SignalHandlerId, WeakRef};
use gtk4::glib::clone::Downgrade;
use gtk4::prelude::{ButtonExt, DrawingAreaExtManual, GdkCairoContextExt, ListItemExt, ObjectExt, WidgetExt};
use relm4::{gtk, RelmObjectExt, Sender};
use relm4::binding::{Binding, BoolBinding, F32Binding};
use relm4::typed_view::column::{RelmColumn, TypedColumnView};
use relm4::typed_view::TypedListItem;

use crate::ui::graph;
use crate::ui::graph::color_binding::RGBABinding;
use crate::ui::swr_worker::Sample;
use crate::ui::util::{BindingLabelColumn, BindingLabelColumnWrapper};

pub(super) struct GraphElement {
    pub(super) visible: BoolBinding,
    pub(super) x_min: F32Binding,
    pub(super) x_max: F32Binding,
    pub(super) y_min: F32Binding,
    pub(super) y_max: F32Binding,
    pub(super) samples: Vec<(f32, f32)>,
    pub(super) color: RGBABinding,
    pub(super) sender: Sender<graph::Input>,
}

impl GraphElement {
    pub(super) fn push_sample(&mut self, sample: Sample) {
        let Sample { index, freq, value } = sample;
        if self.samples.len() <= index {
            self.samples.resize(index + 1, (0.0, 0.0));
        }
        self.samples[sample.index] = (freq, value);
        let mut x_max = self.x_max.guard();
        let mut x_min = self.x_min.guard();
        let mut y_max = self.y_max.guard();
        let mut y_min = self.y_min.guard();
        *x_max = x_max.max(freq);
        *x_min = x_min.min(freq);
        *y_max = y_max.max(value);
        *y_min = y_min.min(value);
    }
}

impl GraphElement {
    pub(super) fn column_view() -> TypedColumnView<GraphElement, MultiSelection> {
        let mut view = TypedColumnView::new();

        view.append_column::<VisibleColumn>();
        view.append_column::<ColorColumn>();
        view.append_column::<BindingLabelColumnWrapper<MinFreqColumn>>();
        view.append_column::<BindingLabelColumnWrapper<MaxFreqColumn>>();
        view.append_column::<DeleteColumn>();

        view
    }

    pub(super) fn iter(view: &TypedColumnView<GraphElement, MultiSelection>) -> ColumnViewIter<Self> {
        ColumnViewIter {
            view,
            index: 0,
        }
    }
}

struct VisibleColumn;

impl RelmColumn for VisibleColumn {
    type Root = gtk::CheckButton;
    type Widgets = ();
    type Item = GraphElement;
    const COLUMN_NAME: &'static str = "";

    fn setup(_list_item: &ListItem) -> (Self::Root, Self::Widgets) {
        let checkbox = gtk::CheckButton::new();
        (checkbox, ())
    }

    fn bind(item: &mut Self::Item, _widgets: &mut Self::Widgets, root: &mut Self::Root) {
        root.add_binding(&item.visible, "active");
    }
}

struct ColorColumn {
    click_handler: GestureClick,
    handles: Option<[SignalHandlerId; 2]>,
    list_item: WeakRef<ListItem>,
}

impl RelmColumn for ColorColumn {
    type Root = gtk::DrawingArea;
    type Widgets = Self;
    type Item = GraphElement;
    const COLUMN_NAME: &'static str = "Color";

    fn setup(list_item: &ListItem) -> (Self::Root, Self::Widgets) {
        let widget = gtk::DrawingArea::new();

        let click_handler = GestureClick::new();

        widget.add_controller(click_handler.clone());

        (widget, Self { click_handler, handles: None, list_item: Downgrade::downgrade(list_item) })
    }

    fn bind(item: &mut Self::Item, widgets: &mut Self::Widgets, root: &mut Self::Root) {
        let color = item.color.clone();
        
        let color2 = color.clone();
        root.set_draw_func(move |_, ctx, w, h| {
            ctx.rectangle(0.0, 0.0, w as f64, h as f64);
            ctx.set_source_color(&color2.get());
            ctx.fill().unwrap()
        });

        let sender = item.sender.clone();
        let list_item = widgets.list_item.clone();

        let click_handler = widgets.click_handler.connect_released(move |_, _, _, _| {
            if let Some(item) = list_item.upgrade() {
                sender.send(graph::Input::ColorPicker(item.position())).unwrap()
            }
        });
        
        let root = root.clone();
        let redraw_handler = color.connect_value_notify(move |_| {
            root.queue_draw();
        });
        
        widgets.handles = Some([click_handler, redraw_handler]);
    }

    fn unbind(item: &mut Self::Item, widgets: &mut Self::Widgets, root: &mut Self::Root) {
        root.set_draw_func(|_, _, _, _| {});
        if let Some([click_handler, redraw_handler]) = widgets.handles.take() {
            widgets.click_handler.disconnect(click_handler);
            item.color.disconnect(redraw_handler)
        }
    }
}

struct MinFreqColumn;

impl BindingLabelColumn for MinFreqColumn {
    type Item = GraphElement;
    type Value = F32Binding;
    type Target = f32;
    const COLUMN_NAME: &'static str = "Start frequency [MHz]";
    const ENABLE_SORT: bool = false;
    const ENABLE_EXPAND: bool = true;

    fn get_cell_value(item: &Self::Item) -> &Self::Value {
        &item.x_min
    }

    fn format_cell_value(value: &f32) -> String {
        format!("{:.3}", value / 1000000.0)
    }
}

struct MaxFreqColumn;

impl BindingLabelColumn for MaxFreqColumn {
    type Item = GraphElement;
    type Value = F32Binding;
    type Target = f32;

    const COLUMN_NAME: &'static str = "Stop frequency [MHz]";
    const ENABLE_SORT: bool = false;
    const ENABLE_EXPAND: bool = true;

    fn get_cell_value(item: &Self::Item) -> &Self::Value {
        &item.x_max
    }

    fn format_cell_value(value: &f32) -> String {
        format!("{:.3}", value / 1000000.0)
    }
}

struct DeleteColumn {
    signal_handle: Option<SignalHandlerId>,
    list_item: WeakRef<ListItem>,
}

impl RelmColumn for DeleteColumn {
    type Root = gtk::Button;
    type Widgets = Self;
    type Item = GraphElement;
    const COLUMN_NAME: &'static str = "";

    fn setup(list_item: &ListItem) -> (Self::Root, Self::Widgets) {
        let widget = gtk::Button::builder()
            .label("Delete")
            .build();

        (widget, Self { signal_handle: None, list_item: Downgrade::downgrade(list_item) })
    }

    fn bind(item: &mut Self::Item, widgets: &mut Self::Widgets, root: &mut Self::Root) {
        let sender = item.sender.clone();
        let list_item = widgets.list_item.clone();

        widgets.signal_handle = Some(root.connect_clicked(move |_| {
            if let Some(item) = list_item.upgrade() {
                sender.emit(graph::Input::Delete(item.position()))
            }
        }));
    }

    fn unbind(_item: &mut Self::Item, widgets: &mut Self::Widgets, root: &mut Self::Root) {
        if let Some(x) = widgets.signal_handle.take() {
            root.disconnect(x);
        }
    }
}

pub struct ColumnViewIter<'a, T: 'static> {
    view: &'a TypedColumnView<T, MultiSelection>,
    index: u32,
}

impl<'a, T: 'static> Iterator for ColumnViewIter<'a, T> {
    type Item = TypedListItem<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.view.get(self.index);
        if result.is_some() {
            self.index += 1;
        }
        result
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.view.len() - self.index;
        (remaining as usize, Some(remaining as usize))
    }
}