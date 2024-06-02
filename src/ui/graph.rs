mod element;

use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use gtk4::{EventControllerMotion, Gesture, MultiSelection, ResponseType};
use gtk4::gdk::RGBA;
use log::{debug, error, trace, warn};
use plotters::coord::ReverseCoordTranslate;
use plotters::prelude::*;
use plotters_cairo::CairoBackend;
use rand::{Rng, thread_rng};
use relm4::abstractions::DrawHandler;
use relm4::binding::{Binding, BoolBinding, F32Binding};
use relm4::prelude::*;
use relm4::prelude::gtk::prelude::*;
use relm4::typed_view::column::TypedColumnView;
use crate::ui::graph::element::GraphElement;

use crate::ui::swr_worker::Sample;

pub struct Graph {
    x_min: f32,
    x_max: f32,
    y_min: f32,
    y_max: f32,
    active: Option<u32>,
    elements: TypedColumnView<GraphElement, MultiSelection>,
    draw_handler: DrawHandler,
    pointer: Option<(f64, f64)>,
    color_picker: Option<u32>,
    last_color: Option<RGBA>,
}

#[derive(Debug)]
pub enum Input {
    Clear {
        x_min: f32,
        x_max: f32,
        y_min: f32,
        y_max: f32,
    },
    Sample(Sample),
    PointerMove(Option<(f64, f64)>),
    Redraw,
    ColorPicker(u32),
    Delete(u32),
    SetColor(Option<RGBColor>),
}

#[relm4::component(pub)]
//noinspection RsSortImplTraitMembers
impl Component for Graph {
    type CommandOutput = ();
    type Input = Input;
    type Output = ();
    type Init = gtk::Window;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            #[local_ref]
            drawing_area -> gtk::DrawingArea {
                set_hexpand: true,
                set_vexpand: true,
                connect_resize[sender] => move |_, _, _| {
                    sender.input(Input::Redraw);
                },
                add_controller= EventControllerMotion {
                    connect_motion[sender] => move |_, x, y| {
                        sender.input(Input::PointerMove(Some((x, y))))
                    },
                    connect_leave => Input::PointerMove(None),
                }
            },
            gtk::ScrolledWindow {
                set_height_request: 150,
                #[local_ref]
                col_view -> gtk::ColumnView {
                    set_hexpand: true,
                }
            },
        },
        #[name(color_picker)]
        gtk::ColorChooserDialog {
            #[watch]
            set_visible: model.color_picker.is_some(),
            set_modal: true,
            set_use_alpha: false,
            set_transient_for: Some(&window),
            #[watch]
            set_rgba?: &model.last_color,
            connect_color_activated[sender] => move |c, color| {
                let r = (color.red() * 255.0) as u8;
                let g = (color.green() * 255.0) as u8;
                let b = (color.blue() * 255.0) as u8;
                sender.input(Input::SetColor(Some(RGBColor(r, g, b))))
            },
            connect_response[sender] => move |c, r| {
                debug!("Color picker response: {:?}", r);
                if r == ResponseType::Cancel {
                    sender.input(Input::SetColor(None));
                    return;
                }
                let color = c.rgba();

                let r = (color.red() * 255.0) as u8;
                let g = (color.green() * 255.0) as u8;
                let b = (color.blue() * 255.0) as u8;
                sender.input(Input::SetColor(Some(RGBColor(r, g, b))))
            },
        }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        self.last_color = None;
        match message {
            Input::Clear {
                x_min: start_freq,
                x_max: stop_freq,
                y_min,
                y_max,
            } => {
                self.x_min = start_freq;
                self.x_max = stop_freq;
                self.y_min = y_min;
                self.y_max = y_max;
                if let Some(active) = self.active {
                    let previous_element = self.elements.get(active).unwrap();
                    let previous_element = previous_element.borrow_mut();
                    previous_element.visible.set(false);
                }
                let (r, g, b) = HSLColor(thread_rng().gen_range(0.0..1.0), 1.0, 0.5).to_backend_color().rgb;
                let element = GraphElement {
                    visible: BoolBinding::new(true),
                    x_min: F32Binding::new(start_freq),
                    x_max: F32Binding::new(stop_freq),
                    y_min: F32Binding::new(y_min),
                    y_max: F32Binding::new(y_max),
                    samples: vec![],
                    color: Rc::new(RefCell::new(RGBColor(r, g, b))),
                    sender: sender.input_sender().clone(),
                };
                element.visible.connect_value_notify(move |_| sender.input(Input::Redraw));
                self.elements.append(element);
                self.active = Some(self.elements.len() - 1)
            }
            Input::Sample(sample) => {
                let Some(active) = self.active else {
                    error!("unexpected sample");
                    return;
                };
                let Some(graph) = self.elements.get(active) else {
                    panic!("graph does not exist");
                };
                let mut graph = graph.borrow_mut();
                graph.push_sample(sample);
            }
            Input::Redraw => {}
            Input::PointerMove(pointer) => {
                self.pointer = pointer;
            }

            Input::ColorPicker(index) => {
                println!("color picker {}", index);
                let RGBColor(r, g, b) = *self.elements.get(index).unwrap().borrow().color.borrow();
                let rgba = RGBA::new(
                    r as f32 / 255.0,
                    g as f32 / 255.0,
                    b as f32 / 255.0,
                    1.0
                );
                self.color_picker = Some(index);
                self.last_color = Some(rgba);
            }
            Input::SetColor(color) => {
                if let Some(i) = self.color_picker.take() {
                    if let Some(color) = color {
                        let elem = self.elements.get(i).unwrap();
                        *elem.borrow_mut().color.borrow_mut() = color;
                        self.elements.view.queue_resize();
                    }
                } else {
                    warn!("color picked for unknown element");
                }
            }
            Input::Delete(index) => {
                self.elements.remove(index);
                if let Some(prev) = self.active.take() {
                    if prev > index {
                        self.active = Some(prev - 1);
                    } else if prev < index {
                        self.active = Some(prev);
                    }
                }
            }
        };
        self.draw();
    }

    fn init(window: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = Self {
            x_min: 0.0,
            x_max: 1000000.0,
            y_min: 0.0,
            y_max: 1.0,
            active: None,
            elements: GraphElement::column_view(),
            draw_handler: DrawHandler::new(),
            pointer: None,
            color_picker: None,
            last_color: None,
        };

        let drawing_area = model.draw_handler.drawing_area();
        let col_view = &model.elements.view;

        let widgets = view_output!();

        ComponentParts {
            model,
            widgets,
        }
    }
}

impl Graph {
    fn draw(&mut self) {
        let size = self.draw_handler.drawing_area().allocation();
        let w = size.width();
        let h = size.height();
        let cx = self.draw_handler.get_context();

        let be = CairoBackend::new(&cx, (w as u32, h as u32)).expect("cairo issue");

        let root = be.into_drawing_area();
        root.fill(&WHITE).unwrap();

        let mut chart = ChartBuilder::on(&root)
            .margin(20)
            .x_label_area_size(40)
            .y_label_area_size(60)
            .build_cartesian_2d(self.x_min..self.x_max, self.y_min..self.y_max).unwrap();

        chart.configure_mesh()
            .x_desc("Frequency [MHz]")
            .x_label_formatter(&|x| format!("{:.2}", x / 1000000.0))
            .y_desc("Voltage [dB]")
            .draw().unwrap();

        for elem in GraphElement::iter(&self.elements) {
            let elem = elem.borrow();

            if !elem.visible.get() { continue; }

            chart
                .draw_series(LineSeries::new(
                    elem.samples.iter().copied(),
                    *elem.color.borrow(),
                )).unwrap()
                .label("main")
                .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));
        }

        if let Some(point) = self.pointer
            .and_then(|(x, y)| chart.as_coord_spec().reverse_translate((x as i32, y as i32)))
            .and_then(|p| self.get_closest(p)) {
            chart.plotting_area().draw(&Cross::new(point, 10, &BLACK)).unwrap();
            root.draw_text(
                &format!("({:.3} MHz, {:.3} dBV)", point.0 / 1000000.0, point.1),
                &("sans-serif", 10, &BLACK).into_text_style(chart.plotting_area()),
                (0, h - 10),
            )
                .unwrap();
        };

        root.present().unwrap();
    }

    fn get_closest(&self, (x, y): (f32, f32)) -> Option<(f32, f32)> {
        let points: Vec<_> = GraphElement::iter(&self.elements).filter_map(|elem| {
            let elem = elem.borrow();

            if !elem.visible.get() { return None; }

            let i = elem.samples.binary_search_by(|this| this.0.total_cmp(&x))
                .unwrap_or_else(|i| i.saturating_sub(1));
            elem.samples.get(i).cloned()
        }).collect();
        points.iter()
            .min_by(|(_, y1), (_, y2)| (y - y1).abs().total_cmp(&(y - y2).abs()))
            .cloned()
    }
}