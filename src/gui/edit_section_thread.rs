use anyhow::Context;
use aviutl2_eframe::egui;

use super::{KeyframesGui, SelectedObjectInfo, TimeControlEditorTarget};

type EditSectionResult = anyhow::Result<(
    aviutl2::generic::EditInfo,
    Option<SelectedObjectInfo>,
    Option<TimeControlEditorTarget>,
    Option<aviutl2::generic::EffectHandle>,
)>;

enum EditSectionMessage {
    Read(Option<TimeControlEditorTarget>),
    Shutdown,
}

pub(super) struct EditSectionThread {
    thread: Option<std::thread::JoinHandle<()>>,
    sender: std::sync::mpsc::Sender<EditSectionMessage>,
    receiver: std::sync::mpsc::Receiver<EditSectionResult>,
    pending: bool,
}

impl EditSectionThread {
    pub(super) fn start(ctx: egui::Context) -> std::io::Result<Self> {
        let (sender, request_receiver) = std::sync::mpsc::channel();
        let (result_sender, receiver) = std::sync::mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("enhanced-tracks-gui-reader".to_owned())
            .spawn(move || {
                while let Ok(message) = request_receiver.recv() {
                    let EditSectionMessage::Read(timecontrol_editor) = message else {
                        break;
                    };
                    let requested_effect = timecontrol_editor.as_ref().map(|target| target.effect);
                    let edit_info = crate::EDIT_HANDLE.get_edit_info();
                    let result = crate::EDIT_HANDLE
                        .call_read_section(|read| {
                            let selected_object_info =
                                KeyframesGui::read_selected_object_info(read)
                                    .context("Failed to update selected object info")?;
                            let timecontrol_editor = KeyframesGui::read_timecontrol_editor_target(
                                timecontrol_editor,
                                read,
                            )
                            .context("Failed to update timecontrol editor target")?;
                            anyhow::Ok((
                                edit_info,
                                selected_object_info,
                                timecontrol_editor,
                                requested_effect,
                            ))
                        })
                        .map_err(anyhow::Error::from)
                        .flatten();
                    if result_sender.send(result).is_err() {
                        break;
                    }
                    ctx.request_repaint();
                }
            })?;
        Ok(Self {
            thread: Some(thread),
            sender,
            receiver,
            pending: false,
        })
    }

    pub(super) fn try_recv(&mut self) -> Option<EditSectionResult> {
        if !self.pending {
            return None;
        }
        match self.receiver.try_recv() {
            Ok(result) => {
                self.pending = false;
                Some(result)
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                panic!("GUI edit section thread disconnected");
            }
        }
    }

    pub(super) fn request(&mut self, timecontrol_editor: Option<TimeControlEditorTarget>) {
        if self.pending {
            return;
        }
        self.sender
            .send(EditSectionMessage::Read(timecontrol_editor))
            .expect("GUI edit section thread must be connected");
        self.pending = true;
    }
}

impl Drop for EditSectionThread {
    fn drop(&mut self) {
        if let Err(error) = self.sender.send(EditSectionMessage::Shutdown) {
            tracing::error!("Failed to stop GUI edit section thread: {:?}", error);
        }
        if let Some(thread) = self.thread.take()
            && let Err(error) = thread.join()
        {
            tracing::error!("Failed to join GUI edit section thread: {:?}", error);
        }
    }
}
