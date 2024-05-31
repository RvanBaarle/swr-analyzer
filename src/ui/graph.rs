use plotters::prelude::*;
use plotters_cairo::CairoBackend;
use relm4::abstractions::DrawHandler;
use relm4::prelude::*;
use relm4::prelude::gtk::prelude::*;

use crate::ui::swr_worker::Sample;

pub struct Graph {
    start_freq: f32,
    stop_freq: f32,
    y_min: f32,
    y_max: f32,
    samples: Vec<(f32, f32)>,
    draw_handler: DrawHandler,
}

#[derive(Debug)]
pub enum Input {
    Clear {
        start_freq: f32,
        stop_freq: f32,
        y_min: f32,
        y_max: f32,
    },
    Sample(Sample),
    Resize,
}

#[relm4::component(pub)]
//noinspection RsSortImplTraitMembers
impl Component for Graph {
    type CommandOutput = ();
    type Input = Input;
    type Output = ();
    type Init = ();

    view! {
        #[root]
        #[name(drawing_area)]
        gtk::DrawingArea {
            connect_resize[sender] => move |_, _, _| {
                sender.input(Input::Resize);
            }
        }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            Input::Clear {
                start_freq,
                stop_freq,
                y_min,
                y_max,
            } => {
                self.start_freq = start_freq;
                self.stop_freq = stop_freq;
                self.y_min = y_min;
                self.y_max = y_max;
            }
            Input::Sample(sample) => {
                if self.samples.len() <= sample.index {
                    self.samples.resize(sample.index + 1, (0.0, 0.0));
                }
                self.samples[sample.index] = (sample.freq, sample.value);
            },
            Input::Resize => {},
        };
        draw_graph(self);
    }

    fn init(_init: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {

        let mut model = Self {
            start_freq: 0.0,
            stop_freq: 1000000.0,
            y_min: 0.0,
            y_max: 1.0,
            samples: vec![],
            draw_handler: DrawHandler::new(),
        };

        model.draw_handler = DrawHandler::new_with_drawing_area(root);

        let root = model.draw_handler.drawing_area();
        
        let widgets = view_output!();
        
        ComponentParts {
            model,
            widgets,
        }
    }
}

fn draw_graph(graph: &mut Graph) {
    let size = graph.draw_handler.drawing_area().allocation();
    let w = size.width();
    let h = size.height();
    let cx = graph.draw_handler.get_context();

    let be = CairoBackend::new(&cx, (w as u32, h as u32)).expect("cairo issue");
    
    let root = be.into_drawing_area();
    root.fill(&WHITE).unwrap();
    
    let mut chart = ChartBuilder::on(&root)
        .caption("Oneshot", ("sans-serif", 30).into_font())
        .margin(5)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(graph.start_freq..graph.stop_freq, graph.y_min..graph.y_max).unwrap();

    chart.configure_mesh()
        .x_desc("Frequency [MHz]")
        .x_label_formatter(&|x| format!("{:.2}", x / 1000000.0))
        .y_desc("Voltage [dB]")
        .draw().unwrap();

    chart
        .draw_series(LineSeries::new(
            graph.samples.iter().copied(),
            &RED,
        )).unwrap();

    root.present().unwrap();
}