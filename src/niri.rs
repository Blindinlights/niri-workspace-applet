// SPDX-License-Identifier: GPL-3.0-only


use cosmic::iced::{
    futures::{SinkExt, Stream},
    Subscription,
};
use log::{debug, error};
use niri_ipc::{socket::Socket, Reply, Workspace};
use tokio::{io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader}, net::{unix::{OwnedReadHalf, OwnedWriteHalf}, UnixStream}};

pub trait NiriSocketExt {
    fn focus_worspace(&mut self, idx: u64);
    fn focus_worspace_up(&mut self);
    fn focus_worspace_down(&mut self);
    fn get_workspace(&mut self) -> Vec<Workspace>;
}
impl NiriSocketExt for Socket {
    fn focus_worspace(&mut self, id: u64) {
        self.send(niri_ipc::Request::Action(
            niri_ipc::Action::FocusWorkspace {
                reference: niri_ipc::WorkspaceReferenceArg::Id(id),
            },
        ))
        .inspect_err(|e| {
            error!("Failed to focus workspace {} : {}", id, e);
        })
        .ok();
    }

    fn focus_worspace_up(&mut self) {
        
        self.send(niri_ipc::Request::Action(
            niri_ipc::Action::FocusWorkspaceUp {},
        ))
        .inspect_err(|e| {
            error!("Failed to focus workspace up : {}", e);
        })
        .ok();
        debug!("Focus up");
    }

    fn focus_worspace_down(&mut self) {
        self.send(niri_ipc::Request::Action(
            niri_ipc::Action::FocusWorkspaceDown {},
        ))
        .inspect_err(|e| {
            error!("Failed to focus workspace down : {}", e);
        })
        .ok();
    }

    fn get_workspace(&mut self) -> Vec<Workspace> {
        let res = self.send(niri_ipc::Request::Workspaces).inspect_err(|e| {
            error!("Failed to get workspace: {}", e);
        });
        if let Ok(Ok(response)) = res {
            match response {
                niri_ipc::Response::Workspaces(w) => return w,
                _ => unreachable!(),
            }
        } else {
            return vec![];
        }
    }
}

pub struct NiriClient {
    writer: OwnedWriteHalf,
    reader: BufReader<OwnedReadHalf>,
}

impl NiriClient {
    /// 连接到指定路径的 Niri Unix domain socket。
    pub async fn connect() ->io::Result<Self> {
        let socket_path=std::env::var(niri_ipc::socket::SOCKET_PATH_ENV).unwrap();
        let stream = UnixStream::connect(socket_path).await?;
        let (read_half, write_half) = stream.into_split();
        let reader = BufReader::new(read_half);

        Ok(Self { writer: write_half, reader })
    }

    /// 发送一个通用的命令并等待回复。
    pub async fn event_stream(
        &mut self,
    ) ->io::Result<Reply> {
        let request = niri_ipc::Request::EventStream;
        let request_json = serde_json::to_string(&request)?;

        self.writer.write_all(request_json.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;


        let mut reply_line = String::new();
         self.reader.read_line(&mut reply_line).await? ;
        let trimmed_reply = reply_line.trim();

        let niri_reply: Reply = serde_json::from_str(trimmed_reply)?;
        Ok(niri_reply)
    }

    pub async fn read_event(&mut self) ->io::Result<niri_ipc::Event> {
        let mut event_line_buffer = String::new();
        self.reader.read_line(&mut event_line_buffer).await?;

        let trimmed_line = event_line_buffer.trim();

        let event: niri_ipc::Event = serde_json::from_str(trimmed_line)?;
        Ok(event)
    }
}
#[derive(Debug, Clone)]
pub enum WorkspaceUpdate {
    WorkspaceChanged(Vec<Workspace>),
    FocusChanged(u64),
}
pub fn sub() -> Subscription<WorkspaceUpdate> {
    Subscription::run(worker)
}
pub fn worker() -> impl Stream<Item = WorkspaceUpdate> {
    cosmic::iced::stream::channel(4, async |mut output| {
        let mut niri_socket =
            NiriClient::connect().await.expect("Event loop :failed to connect to niri socket");

        let reply = niri_socket.event_stream().await.expect("");
        
        if matches!(reply, Ok(niri_ipc::Response::Handled)) {
            while let Ok(event) = niri_socket.read_event().await {
                match event {
                    niri_ipc::Event::WorkspacesChanged { workspaces } => {
                        output
                            .send(WorkspaceUpdate::WorkspaceChanged(workspaces))
                            .await
                            .expect("Error send message");
                    }
                    niri_ipc::Event::WorkspaceActivated { id, focused } => {
                        if focused {
                            output
                                .send(WorkspaceUpdate::FocusChanged(id))
                                .await
                                .inspect_err(|e| {
                                    error!("{}", e);
                                })
                                .unwrap();
                        }
                    }
                    _ => {
                        // debug!("niri event:{:?}",event);
                    }
                }
            }
        }
    })
}
