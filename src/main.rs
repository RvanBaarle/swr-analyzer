use relm4::{RELM_THREADS, RelmApp};

use ui::App;

mod protocol;
mod ui;

fn main() {
    let app = RelmApp::new("nl.vbaarle.ruben.swranalyzer");
    ui::log::Logger::init();
    app.run::<App>(());
}

