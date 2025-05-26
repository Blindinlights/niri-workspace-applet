// SPDX-License-Identifier: GPL-3.0-only

use cosmic::app::{Core, Task};
use cosmic::applet::cosmic_panel_config::PanelAnchor;
use cosmic::iced::{Alignment, Background, Border, Length, Limits, Subscription};
use cosmic::iced_widget::{button, column, row};
use cosmic::widget::{autosize, container, horizontal_space, vertical_space};
use cosmic::{Application, Element, Theme};
use log::debug;
use niri_ipc::socket::Socket;
use niri_ipc::Workspace;

use crate::niri::{self, NiriSocketExt, WorkspaceUpdate};

// use crate::fl;
pub struct NiriWorkspaceApplet {
    core: Core,
    workspaces: Vec<Workspace>,
    focused: u64,
    socket: Socket,
}
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Message {
    WorkspaceUpdated(WorkspaceUpdate),
    FocusWorkspace(u64),
    FocusWorkspaceDown,
    FocusWorkspaceUp,
    Ping,
}

impl Application for NiriWorkspaceApplet {
    type Executor = cosmic::executor::multi::Executor;

    type Flags = ();

    type Message = Message;

    const APP_ID: &'static str = "com.github.blindinlights.NiriWorkSpaceApplet";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }
    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        let mut socket = Socket::connect().expect("Failed to connect to niri socket.");
        let mut workspaces = socket.get_workspace();
        workspaces.sort_by(|w1, w2| w1.idx.cmp(&w2.idx));
        let app = NiriWorkspaceApplet {
            core,
            socket,
            workspaces,
            focused: 0,
        };
        debug!("App init");
        (app, Task::none())
    }

    fn view(&self) -> Element<Self::Message> {
        if self.workspaces.is_empty() {
            return row![].padding(8).into();
        }
        let horizontal = matches!(
            self.core.applet.anchor,
            PanelAnchor::Top | PanelAnchor::Bottom
        );
        let suggested_total = self.core.applet.suggested_size(false).0
            + self.core.applet.suggested_padding(false) * 2;
        let suggested_window_size = self.core.applet.suggested_window_size();

        let buttons = self.workspaces.iter().filter_map(|w| {
            let content = self
                .core
                .applet
                .text(w.name.clone().unwrap_or(w.idx.to_string()))
                .font(cosmic::font::bold());
            let (width, height) = if self.core.applet.is_horizontal() {
                (suggested_total as f32, suggested_window_size.1.get() as f32)
            } else {
                (suggested_window_size.0.get() as f32, suggested_total as f32)
            };
            let content = row!(content, vertical_space().height(Length::Fixed(height)))
                .align_y(Alignment::Center);

            let content = column!(content, horizontal_space().width(Length::Fixed(width)))
                .align_x(Alignment::Center);

            let btn = button(
                container(content)
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center),
            )
            .padding(if horizontal {
                [0, self.core.applet.suggested_padding(true)]
            } else {
                [self.core.applet.suggested_padding(true), 0]
            })
            .on_press(Message::FocusWorkspace(w.id))
            .padding(2);
            Some(
                btn.class(if w.is_focused {
                    cosmic::theme::iced::Button::Primary
                } else {
                    let appearance = |theme: &Theme| {
                        let cosmic = theme.cosmic();
                        button::Style {
                            background: None,
                            border: Border {
                                radius: cosmic.radius_xl().into(),
                                ..Default::default()
                            },
                            border_radius: cosmic.radius_xl().into(),
                            text_color: theme.current_container().component.on.into(),
                            ..button::Style::default()
                        }
                    };
                    cosmic::theme::iced::Button::Custom(Box::new(
                        move |theme, status| match status {
                            button::Status::Active => appearance(theme),
                            button::Status::Hovered => button::Style {
                                background: Some(Background::Color(
                                    theme.current_container().component.hover.into(),
                                )),
                                border: Border {
                                    radius: theme.cosmic().radius_xl().into(),
                                    ..Default::default()
                                },
                                ..appearance(theme)
                            },
                            button::Status::Pressed | button::Status::Disabled => appearance(theme),
                        },
                    ))
                })
                .into(),
            )
        });
        let layout_section: Element<_> = row(buttons).spacing(4).into();
        let mut limits = Limits::NONE.min_width(1.).min_height(1.);
        if let Some(b) = self.core.applet.suggested_bounds {
            if b.width as i32 > 0 {
                limits = limits.max_width(b.width);
            }
            if b.height as i32 > 0 {
                limits = limits.max_height(b.height);
            }
        }

        autosize::autosize(
            container(layout_section).padding(0),
            cosmic::widget::Id::new("autosize-main"),
        )
        .limits(limits)
        .into()
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::WorkspaceUpdated(update) => match update {
                WorkspaceUpdate::WorkspaceChanged(workspaces) => {
                    self.workspaces = workspaces;
                    self.workspaces.sort_by(|w1, w2| w1.idx.cmp(&w2.idx));
                }
                WorkspaceUpdate::FocusChanged(id) => {
                    self.focused = id;
                    for w in &mut self.workspaces {
                        w.is_focused = w.id == id;
                    }
                }
            },
            Message::FocusWorkspace(idx) => {
                self.socket.focus_worspace(idx);
            }
            Message::FocusWorkspaceDown => {
                self.socket.focus_worspace_down();
            }
            Message::FocusWorkspaceUp => {
                self.socket.focus_worspace_up();
            }
            Message::Ping => {
                debug!("Update Pong!!")
            }
        }
        Task::none()
    }
    fn subscription(&self) -> cosmic::iced::Subscription<Self::Message> {
        Subscription::batch([niri::sub().map(Message::WorkspaceUpdated)])
    }

    fn style(&self) -> Option<cosmic::iced_runtime::Appearance> {
        Some(cosmic::applet::style())
    }
}
