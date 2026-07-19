use aviutl2_eframe::egui::{self, IntoAtoms as _};

static PRESSED_FLAG_ID_EXT: &str = "was_pressed_in_last_frame";

// モディファイアなしのLShiftを押す方法はないはず...
pub static NO_SHORTCUT: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::ShiftLeft);

pub fn parse_accesskey(text: &str) -> (egui::Atoms<'static>, egui::KeyboardShortcut) {
    let mut layout = egui::text::LayoutJob::default();
    let mut chars = text.chars();
    let mut shortcut_key = NO_SHORTCUT;
    let mut has_shortcut = false;
    while let Some(c) = chars.next() {
        if c == '&' {
            if let Some(next_c) = chars.next() {
                has_shortcut = true;
                if next_c == '&' {
                    layout.append("&", 0.0, {
                        egui::TextFormat {
                            color: egui::Color32::PLACEHOLDER,
                            ..Default::default()
                        }
                    });
                } else {
                    layout.append(
                        &next_c.to_string(),
                        0.0,
                        egui::TextFormat {
                            underline: egui::Stroke::new(1.0, egui::Color32::PLACEHOLDER),
                            color: egui::Color32::PLACEHOLDER,
                            ..Default::default()
                        },
                    );
                    shortcut_key = egui::Key::from_name(&next_c.to_string())
                        .map(|key| egui::KeyboardShortcut::new(egui::Modifiers::NONE, key))
                        .unwrap_or(NO_SHORTCUT);
                }
            }
        } else {
            layout.append(&c.to_string(), 0.0, {
                egui::TextFormat {
                    color: egui::Color32::PLACEHOLDER,
                    ..Default::default()
                }
            })
        }
    }

    if has_shortcut {
        (layout.into_atoms(), shortcut_key)
    } else {
        (text.into_atoms(), NO_SHORTCUT)
    }
}

pub struct AccessKeyContext {
    ctx: egui::Context,
    shortcuts: Vec<(egui::KeyboardShortcut, egui::Id)>,
    level: usize,
    max_level: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}
impl AccessKeyContext {
    pub fn root(ctx: &egui::Context) -> Self {
        Self {
            ctx: ctx.clone(),
            shortcuts: vec![],
            level: 0,
            max_level: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    pub fn child(&mut self) -> Self {
        self.max_level
            .fetch_max(self.level + 1, std::sync::atomic::Ordering::Relaxed);
        Self {
            ctx: self.ctx.clone(),
            shortcuts: vec![],
            level: self.level + 1,
            max_level: self.max_level.clone(),
        }
    }

    pub fn add_button<'b>(
        &mut self,
        ui: &mut egui::Ui,
        shortcut_key: egui::KeyboardShortcut,
        button: egui::Button<'b>,
    ) -> egui::Response {
        self.add_button_enabled(ui, true, shortcut_key, button)
    }

    pub fn add_button_enabled<'b>(
        &mut self,
        ui: &mut egui::Ui,
        enabled: bool,
        shortcut_key: egui::KeyboardShortcut,
        button: egui::Button<'b>,
    ) -> egui::Response {
        let id = ui.next_auto_id();

        self.process_last_frame_pressed(ui, shortcut_key, id);

        ui.add_enabled(
            enabled,
            button.shortcut_text(if shortcut_key != NO_SHORTCUT {
                ui.format_shortcut(&shortcut_key).into_atoms()
            } else {
                egui::AtomKind::Empty.into_atoms()
            }),
        )
    }

    pub fn add_menu_button<'b, R>(
        &mut self,
        ui: &mut egui::Ui,
        shortcut_key: egui::KeyboardShortcut,
        button_atoms: impl egui::IntoAtoms<'b>,
        add_contents: impl FnOnce(&mut egui::Ui, &mut AccessKeyContext) -> R,
    ) -> egui::InnerResponse<Option<R>> {
        let id = ui.next_auto_id();

        self.process_last_frame_pressed(ui, shortcut_key, id);

        ui.menu_button(
            (
                button_atoms,
                if shortcut_key != NO_SHORTCUT {
                    egui::AtomKind::text(
                        egui::RichText::new(format!(" ({})", ui.format_shortcut(&shortcut_key)))
                            .weak(),
                    )
                    .into_atoms()
                } else {
                    egui::AtomKind::Empty.into_atoms()
                },
            ),
            |ui| add_contents(ui, &mut self.child()),
        )
    }

    pub fn add_checkbox<'b>(
        &mut self,
        ui: &mut egui::Ui,
        shortcut_key: egui::KeyboardShortcut,
        value: &'b mut bool,
        atoms: impl egui::IntoAtoms<'b>,
    ) -> egui::Response {
        self.add_checkbox_enabled(ui, true, shortcut_key, value, atoms)
    }

    pub fn add_checkbox_enabled<'b>(
        &mut self,
        ui: &mut egui::Ui,
        enabled: bool,
        shortcut_key: egui::KeyboardShortcut,
        value: &'b mut bool,
        atoms: impl egui::IntoAtoms<'b>,
    ) -> egui::Response {
        let id = ui.next_auto_id();

        self.process_last_frame_pressed(ui, shortcut_key, id);

        ui.add_enabled(
            enabled,
            egui::Checkbox::new(
                value,
                (
                    atoms,
                    if shortcut_key != NO_SHORTCUT {
                        egui::AtomKind::text(
                            egui::RichText::new(format!(
                                " ({})",
                                ui.format_shortcut(&shortcut_key)
                            ))
                            .weak(),
                        )
                        .into_atoms()
                    } else {
                        egui::AtomKind::Empty.into_atoms()
                    },
                ),
            ),
        )
    }

    fn process_last_frame_pressed(
        &mut self,
        ui: &mut egui::Ui,
        shortcut_key: egui::KeyboardShortcut,
        id: egui::Id,
    ) {
        // https://github.com/emilk/egui/issues/2831#issuecomment-3685211018
        let was_pressed_in_last_frame = ui
            .data_mut(|data| data.remove_temp::<()>(id.with(PRESSED_FLAG_ID_EXT)))
            .is_some();

        if was_pressed_in_last_frame {
            ui.memory_mut(|mem| mem.request_focus(id));
            ui.input_mut(|i| {
                i.events.push(egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                })
            });

            ui.scroll_to_rect(
                egui::Rect::from_pos(ui.next_widget_position())
                    .expand2(egui::Vec2::new(0.0, ui.spacing().interact_size.y)),
                None,
            );
        }
        self.shortcuts.push((shortcut_key, id));
    }
}

impl Drop for AccessKeyContext {
    fn drop(&mut self) {
        if self.level != self.max_level.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }

        let mut pressed_id: Option<egui::Id> = None;
        for (shortcut, id) in &self.shortcuts {
            if self.ctx.input_mut(|i| i.consume_shortcut(shortcut)) {
                pressed_id = Some(*id);
                break;
            }
        }

        if let Some(id) = pressed_id {
            self.ctx
                .data_mut(|data| data.insert_temp(id.with(PRESSED_FLAG_ID_EXT), ()));
        }
    }
}
