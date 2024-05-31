use relm4::RelmApp;

use ui::App;

mod protocol;
mod ui;

fn main() {
    let app = RelmApp::new("nl.vbaarle.ruben.swranalyzer");
    app.run::<App>(());
}

