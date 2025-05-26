// SPDX-License-Identifier: GPL-3.0-only
mod app;
mod core;
mod niri;
use app::NiriWorkspaceApplet;

fn main() -> cosmic::iced::Result {
    env_logger::init();
    cosmic::applet::run::<NiriWorkspaceApplet>(())
}
