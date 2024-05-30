use relm4::RelmApp;

use crate::ui::relm::App;

mod protocol;
mod ui;

fn main() {
    let app = RelmApp::new("nl.vbaarle.ruben.swranalyzer");
    ui::log::Logger::init();
    app.run_async::<App>(());
}

