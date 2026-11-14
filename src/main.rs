#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use relm4::RelmApp;
use simpad_control_panel_rs::ui::AppModel;

fn main() {
    let app = RelmApp::new("net.bysb.simpad.controlpanel");
    app.run::<AppModel>(());
}
