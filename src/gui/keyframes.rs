#![allow(clippy::too_many_arguments)]
use super::*;
use aviutl2_eframe::egui;
use egui::epaint::{Mesh, Shape};

static SECTION_SEPARATOR_HITBOX_WEIGHT: f32 = 4.0;
static KEYFRAME_TIMELINE_FADE_WIDTH: f32 = 24.0;

struct EasingSearchItem<'a> {
    easing: &'a crate::keyframe::Easing,
    text: String,
}

impl AsRef<str> for EasingSearchItem<'_> {
    fn as_ref(&self) -> &str {
        &self.text
    }
}

impl KeyframesGui {
    pub fn render_selected_object_info(&mut self, ui: &mut egui::Ui) {
        let Some(selected_object_info) = self.selected_object_info.clone() else {
            ui.label(aviutl2::config::translate(
                "オブジェクトが選択されていません。",
            ));
            ui.separator();

            ui.horizontal_wrapped(|ui| {
                ui.hyperlink_to(
                    egui::RichText::new("enhanced_tracks.aux2").size(20.0),
                    "https://github.com/sevenc-nanashi/enhanced_tracks.aux2",
                );
                ui.label(format!("v{}", env!("CARGO_PKG_VERSION")));
            });
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label("developed by ");
                ui.hyperlink_to(
                    egui::RichText::new("Nanashi.")
                        .color(egui::Color32::from_rgb(0x48, 0xb0, 0xd5)),
                    "https://sevenc7c.com",
                );
            });
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label("powered by ");
                ui.hyperlink_to(
                    egui::RichText::new("aviutl2-rs")
                        .color(egui::Color32::from_rgb(0xf8, 0x52, 0x07)),
                    "https://github.com/sevenc-nanashi/aviutl2-rs",
                );
            });
            return;
        };
        // ui.label(format!("Selected Object: {}", selected_object_info.name));
        self.handle_keyframe_timeline_input(ui, ui.clip_rect());
        if ui
            .add(
                egui::Label::new(
                    egui::RichText::new(format!(
                        "{} ({}f - {}f)",
                        selected_object_info.name,
                        selected_object_info.frames[0] + 1,
                        selected_object_info.frames.last().unwrap()
                    ))
                    .color(
                        if crate::module::DEBUG_MODE.load(std::sync::atomic::Ordering::Relaxed) {
                            GUI_COLORS.log_warn
                        } else {
                            GUI_COLORS.text
                        },
                    ),
                )
                .sense(egui::Sense::click()),
            )
            .clicked()
            && ui.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.modifiers.alt)
        {
            self.debug_counter += 1;
            if self.debug_counter >= 5 {
                self.debug_view = true;
            }
        }
        if let Some(info) = self.edit_info {
            ui.push_id(selected_object_info.handle, |ui| {
                for effect in &selected_object_info.effects {
                    self.render_effect_info(ui, &info, &selected_object_info, effect);
                }
            });
        }
    }

    fn render_effect_info(
        &mut self,
        ui: &mut egui::Ui,
        info: &aviutl2::generic::EditInfo,
        object: &SelectedObjectInfo,
        effect: &EffectInfo,
    ) {
        egui::containers::CollapsingHeader::new(crate::utils::get_translated_effect_name(
            &effect.name,
        ))
        .id_salt(effect.handle)
        .enabled(!effect.keyframe_tracks.is_empty())
        .open(effect.keyframe_tracks.is_empty().then_some(false))
        .show(ui, |ui| {
            for track in effect.keyframe_tracks.values() {
                ui.push_id(&track.names, |ui| {
                    self.render_keyframe_track_info(ui, info, object, effect, &track.params, track);
                });
            }
        });
    }

    fn render_keyframe_track_info(
        &mut self,
        ui: &mut egui::Ui,
        info: &aviutl2::generic::EditInfo,
        object: &SelectedObjectInfo,
        effect: &EffectInfo,
        params: &crate::KeyframeTrackParams,
        track: &KeyframeTrackInfo,
    ) {
        ui.horizontal_wrapped(|ui| {
            for name in &track.names {
                ui.menu_button(
                    crate::utils::get_translated_effect_param_name(&effect.name, name),
                    |ui| {
                        if ui
                            .add_enabled(
                                track.names.len() > 1,
                                egui::Button::new(aviutl2::config::translate("分離")),
                            )
                            .clicked()
                        {
                            self.detach_keyframe_track(object, effect, params, track, name);
                        }
                    },
                );
            }
        });
        let (response, painter) = ui.allocate_painter(
            ui.available_size().tap_mut(|s| {
                s.y = 24.0;
            }),
            aviutl2_eframe::egui::Sense::hover(),
        );
        let (current_object_color, selected_object_color) = get_colors(&effect.effect_type);
        let visible_width_ratio = self.keyframe_timeline_view.width();
        let num_divisions = (response.rect.width() / visible_width_ratio) as usize / 10;
        if num_divisions == 0 {
            return;
        }

        let total_frames = object.frames.last().unwrap() - object.frames.first().unwrap();
        self.render_track_background(
            &painter,
            response.rect,
            self.keyframe_timeline_view,
            &current_object_color,
            num_divisions,
        );

        let keyframes = &crate::KEYFRAMES.get(params).map_or_else(
            || crate::keyframe::Keyframes::new(object.frames.len()),
            |keyframes| keyframes.clone(),
        );
        let sections = Self::track_sections(object, total_frames);
        if sections.len() != keyframes.keyframes.len() - 1 {
            return;
        }

        self.render_keyframe_section_interactions(
            ui,
            &painter,
            response.rect,
            self.keyframe_timeline_view,
            effect,
            params,
            track,
            keyframes,
            &sections,
            selected_object_color,
        );
        self.render_easing_labels(
            ui,
            &painter,
            response.rect,
            self.keyframe_timeline_view,
            object,
            keyframes,
            total_frames,
        );
        self.render_midpoint_lines(
            ui,
            &painter,
            response.rect,
            self.keyframe_timeline_view,
            object,
            effect,
            track,
            keyframes,
            total_frames,
        );
        self.render_frame_cursor(
            &painter,
            info,
            object,
            response.rect,
            self.keyframe_timeline_view,
            total_frames,
        );
        self.render_keyframe_timeline_edge_fades(
            &painter,
            response.rect,
            self.keyframe_timeline_view,
        );
    }

    fn handle_keyframe_timeline_input(&mut self, ui: &egui::Ui, input_rect: egui::Rect) {
        let (scroll_delta, zoom_delta, modifiers, pointer_pos) = ui.input(|i| {
            (
                i.smooth_scroll_delta(),
                i.zoom_delta(),
                i.modifiers,
                i.pointer.hover_pos(),
            )
        });
        let Some(pointer_pos) = pointer_pos else {
            return;
        };
        if !input_rect.contains(pointer_pos) {
            return;
        }

        let mut view = self.keyframe_timeline_view;

        if modifiers.ctrl {
            let zoom_factor = if (zoom_delta - 1.0).abs() > f32::EPSILON {
                zoom_delta
            } else if scroll_delta.y.abs() > f32::EPSILON {
                (scroll_delta.y * 0.01).exp()
            } else {
                1.0
            };
            if (zoom_factor - 1.0).abs() > f32::EPSILON {
                let anchor_ratio =
                    ((pointer_pos.x - input_rect.left()) / input_rect.width()).clamp(0.0, 1.0);
                let anchor = view.left + anchor_ratio * view.width();
                view = view.zoom_at(anchor, zoom_factor);
            }
        } else {
            let scroll_x = if modifiers.shift {
                if scroll_delta.y.abs() > f32::EPSILON {
                    -scroll_delta.y
                } else {
                    -scroll_delta.x
                }
            } else {
                scroll_delta.x
            };
            if scroll_x.abs() > f32::EPSILON && input_rect.width() > f32::EPSILON {
                view = view.translate(scroll_x / input_rect.width() * view.width());
            }
        }

        self.keyframe_timeline_view = view;
    }

    fn render_track_background(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        view: KeyframeTimelineView,
        current_object_color: &[egui::Color32],
        num_divisions: usize,
    ) {
        let width_per_section = rect.width() / num_divisions as f32;
        for i in 0..num_divisions {
            let mut section_rect = rect;
            section_rect.set_left(rect.left() + i as f32 * width_per_section);
            section_rect.set_right((section_rect.left() + width_per_section).min(rect.right()));
            let position = view.left + i as f32 / num_divisions as f32 * view.width();
            let color = current_object_color[position.floor() as usize].lerp_to_gamma(
                current_object_color
                    [(position.ceil() as usize).min(current_object_color.len() - 1)],
                position.fract(),
            );
            if i > 0 {
                // たまに境目ができてしまうのでちょっとだけ重ねる
                section_rect.set_left(section_rect.left() - 1.0);
            }
            painter.rect_filled(section_rect, 0.0, color);
        }
    }

    fn render_keyframe_timeline_edge_fades(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        view: KeyframeTimelineView,
    ) {
        let fade_width = KEYFRAME_TIMELINE_FADE_WIDTH.min(rect.width() / 2.0);
        if fade_width <= f32::EPSILON {
            return;
        }

        if view.left > f32::EPSILON {
            let fade_rect = egui::Rect::from_min_max(
                rect.left_top(),
                egui::pos2(rect.left() + fade_width, rect.bottom()),
            );
            Self::paint_horizontal_fade(painter, fade_rect, true);
        }
        if view.right < 1.0 - f32::EPSILON {
            let fade_rect = egui::Rect::from_min_max(
                egui::pos2(rect.right() - fade_width, rect.top()),
                rect.right_bottom(),
            );
            Self::paint_horizontal_fade(painter, fade_rect, false);
        }
    }

    fn paint_horizontal_fade(painter: &egui::Painter, rect: egui::Rect, from_left: bool) {
        let background = GUI_COLORS.object_section_ignored;
        let transparent = egui::Color32::from_rgba_unmultiplied(
            background.r(),
            background.g(),
            background.b(),
            0,
        );
        let (left_color, right_color) = if from_left {
            (background, transparent)
        } else {
            (transparent, background)
        };

        let mut mesh = Mesh::default();
        let idx = mesh.vertices.len() as u32;
        mesh.colored_vertex(rect.left_top(), left_color);
        mesh.colored_vertex(rect.right_top(), right_color);
        mesh.colored_vertex(rect.left_bottom(), left_color);
        mesh.colored_vertex(rect.right_bottom(), right_color);
        mesh.add_triangle(idx, idx + 1, idx + 2);
        mesh.add_triangle(idx + 2, idx + 1, idx + 3);
        painter.add(Shape::mesh(mesh));
    }

    fn track_sections(object: &SelectedObjectInfo, total_frames: usize) -> Vec<(usize, f32, f32)> {
        let mut sections = vec![];
        for i in 0..object.frames.len() - 1 {
            let left_position = (object.frames[i] - object.frames[0]) as f32 / total_frames as f32;
            let right_position =
                (object.frames[i + 1] - object.frames[0]) as f32 / total_frames as f32;
            sections.push((i, left_position, right_position));
        }
        sections
    }

    fn render_keyframe_section_interactions(
        &mut self,
        ui: &mut egui::Ui,
        painter: &egui::Painter,
        track_rect: egui::Rect,
        view: KeyframeTimelineView,
        effect: &EffectInfo,
        params: &crate::KeyframeTrackParams,
        track: &KeyframeTrackInfo,
        keyframes: &crate::keyframe::Keyframes,
        sections: &[(usize, f32, f32)],
        selected_object_color: egui::Color32,
    ) {
        let crate::keyframe::Keyframe::Easing(ref initial_kf_info) = keyframes.keyframes[0] else {
            unreachable!();
        };
        let mut kf_info = initial_kf_info;

        for (i, section) in sections.iter().enumerate() {
            if let crate::keyframe::Keyframe::Easing(ref new_kf_info) =
                keyframes.keyframes[section.0]
            {
                kf_info = new_kf_info;
            }

            let rect = Self::section_rect(track_rect, view, section.1, section.2);
            let Some(clipped_rect) = Self::clip_rect_horizontally(rect, track_rect) else {
                continue;
            };
            let shrinked_rect = {
                let mut rect2 = clipped_rect;
                if i > 0 {
                    rect2.set_left(rect2.left() + SECTION_SEPARATOR_HITBOX_WEIGHT / 2.0);
                }
                if i < sections.len() - 1 {
                    rect2.set_right(rect2.right() - SECTION_SEPARATOR_HITBOX_WEIGHT / 2.0);
                }
                if !rect2.is_positive() {
                    continue;
                }
                rect2
            };
            let response = ui
                .interact(
                    shrinked_rect,
                    ui.id().with("section").with(section.0),
                    aviutl2_eframe::egui::Sense::click(),
                )
                .on_hover_text(Self::easing_hover_text(kf_info));
            if response.hovered() {
                painter.rect_filled(clipped_rect, 0.0, selected_object_color);
            }

            if response.double_clicked()
                && let crate::keyframe::Keyframe::Easing(ref current_kf_info) =
                    keyframes.keyframes[section.0]
                && crate::EASINGS
                    .read()
                    .unwrap()
                    .get(&current_kf_info.easing)
                    .is_some_and(|easing| easing.has_timecontrol)
            {
                self.open_timecontrol_editor(params, effect, track, section.0, current_kf_info);
                tracing::info!(
                    "Opening time control dialog by double click for section {} of track {:?} in effect {:?}",
                    section.0,
                    track.names,
                    effect.name
                );
            }

            egui::containers::Popup::menu(&response)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .show(|ui| {
                    self.show_easing_menu(
                        ui,
                        keyframes,
                        params,
                        effect,
                        track,
                        section.0,
                        "",
                        |new_keyframes| {
                            Self::update_track_keyframes(effect, track, section.0, new_keyframes);
                        },
                    );
                });
        }
    }

    fn open_timecontrol_editor(
        &mut self,
        params: &crate::KeyframeTrackParams,
        effect: &EffectInfo,
        track: &KeyframeTrackInfo,
        keyframe_index: usize,
        keyframe: &crate::keyframe::EasingKeyframeInfo,
    ) {
        self.timecontrol_editor = Some(TimeControlEditorTarget {
            params: *params,
            keyframe_index,
            effect: effect.handle,
            effect_name: effect.name.clone(),
            track_names: track.names.clone(),
            timecontrol: keyframe.timecontrol.clone(),
            selected_point: 0,
            context_menu_position: None,
            preset_panel_width: f32::NAN,
            visible_y_bounds: None,
            drag_scroll_y_bounds: None,
            dirty: false,
        });
    }

    fn section_rect(
        track_rect: egui::Rect,
        view: KeyframeTimelineView,
        left: f32,
        right: f32,
    ) -> egui::Rect {
        let mut rect = track_rect;
        rect.set_left(track_rect.left() + (left - view.left) / view.width() * track_rect.width());
        rect.set_right(track_rect.left() + (right - view.left) / view.width() * track_rect.width());
        rect
    }

    fn clip_rect_horizontally(rect: egui::Rect, clip: egui::Rect) -> Option<egui::Rect> {
        if rect.right() <= clip.left() || clip.right() <= rect.left() {
            return None;
        }
        let mut clipped = rect;
        clipped.set_left(clipped.left().max(clip.left()));
        clipped.set_right(clipped.right().min(clip.right()));
        Some(clipped)
    }

    fn easing_hover_text(kf_info: &crate::keyframe::EasingKeyframeInfo) -> String {
        if kf_info.params.is_empty() {
            return kf_info.easing.clone();
        }

        format!(
            "{}：{}",
            kf_info.easing,
            kf_info
                .params
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn update_track_keyframes(
        effect: &EffectInfo,
        track: &KeyframeTrackInfo,
        section_index: usize,
        new_keyframes: crate::keyframe::Keyframes,
    ) -> Option<crate::KeyframeTrackParams> {
        tracing::debug!(
            "Updating keyframe {:?} of track {:?} in effect {:?} to {:?}",
            section_index,
            track.names,
            effect.name,
            &new_keyframes
        );
        let new_params =
            Self::update_keyframes_for_tracks(effect.handle, &track.names, new_keyframes);
        match new_params {
            Ok(new_params) => {
                tracing::info!(
                    "Updated keyframe track params for section {} of track {:?} in effect {:?} to {:?}",
                    section_index,
                    track.names,
                    effect.name,
                    new_params
                );
                Some(new_params)
            }
            Err(e) => {
                tracing::error!(
                    "Failed to update keyframe track params for section {} of track {:?} in effect {:?}: {:?}",
                    section_index,
                    track.names,
                    effect.name,
                    e
                );
                None
            }
        }
    }

    fn render_easing_labels(
        &self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        track_rect: egui::Rect,
        view: KeyframeTimelineView,
        object: &SelectedObjectInfo,
        keyframes: &crate::keyframe::Keyframes,
        total_frames: usize,
    ) {
        for (i, frame) in object.frames.iter().enumerate() {
            if i == object.frames.len() - 1 {
                continue;
            }
            let easing = match keyframes.keyframes[i] {
                crate::keyframe::Keyframe::Easing(ref easing) => {
                    if crate::EASINGS
                        .read()
                        .unwrap()
                        .get(&easing.easing)
                        .is_some_and(|e| e.has_timecontrol)
                    {
                        format!(
                            "🕒 {}",
                            crate::utils::get_translated_effect_name(&easing.easing)
                        )
                    } else {
                        crate::utils::get_translated_effect_name(&easing.easing)
                    }
                }
                crate::keyframe::Keyframe::Midpoint => "〃".to_string(),
                _ => continue,
            };
            let left_position = (*frame - object.frames[0]) as f32 / total_frames as f32;
            let right_position =
                (object.frames[i + 1] - object.frames[0]) as f32 / total_frames as f32;
            let Some(mut rect) = Self::clip_rect_horizontally(
                Self::section_rect(track_rect, view, left_position, right_position),
                track_rect,
            ) else {
                continue;
            };
            rect.set_left(rect.left() + ui.spacing().button_padding.x);

            let mut layout = egui::text::LayoutJob::default();
            layout.append(
                &easing,
                0.0,
                egui::TextFormat {
                    font_id: egui::FontId::default(),
                    color: GUI_COLORS.text,
                    ..Default::default()
                },
            );
            layout.wrap = egui::text::TextWrapping::truncate_at_width(rect.width());
            let galley = painter.layout_job(layout);
            painter.galley(
                rect.left_center().tap_mut(|pos| {
                    pos.y -= galley.size().y / 2.0;
                }),
                galley,
                GUI_COLORS.text,
            );
        }
    }

    fn render_midpoint_lines(
        &self,
        ui: &mut egui::Ui,
        painter: &egui::Painter,
        track_rect: egui::Rect,
        view: KeyframeTimelineView,
        object: &SelectedObjectInfo,
        effect: &EffectInfo,
        track: &KeyframeTrackInfo,
        keyframes: &crate::keyframe::Keyframes,
        total_frames: usize,
    ) {
        for (i, frame) in object.frames.iter().enumerate() {
            if i == 0 || i == object.frames.len() - 1 {
                continue;
            }
            let position = (*frame - object.frames.first().unwrap()) as f32 / total_frames as f32;
            if position < view.left || view.right < position {
                continue;
            }
            let mut rect = track_rect;
            rect.set_left(
                rect.left() + (position - view.left) / view.width() * track_rect.width() - 1.0,
            );
            rect.set_right(rect.left() + 1.0);
            let mut click_rect = rect;
            click_rect.set_left(click_rect.left() - SECTION_SEPARATOR_HITBOX_WEIGHT / 2.0);
            click_rect.set_right(click_rect.right() + SECTION_SEPARATOR_HITBOX_WEIGHT / 2.0);
            let click = ui.interact(
                click_rect,
                ui.id().with("separator").with(i),
                aviutl2_eframe::egui::Sense::click(),
            );
            if click.hovered() {
                painter.rect_filled(click_rect, 0.0, get_colors(&effect.effect_type).1);
            }
            if click.clicked() {
                let mut new_keyframes = keyframes.clone();
                new_keyframes.keyframes[i] =
                    if matches!(keyframes.keyframes[i], crate::keyframe::Keyframe::Ignored) {
                        crate::keyframe::Keyframe::Midpoint
                    } else {
                        crate::keyframe::Keyframe::Ignored
                    };
                Self::update_track_keyframes(effect, track, i, new_keyframes);
            }
            click.on_hover_text(aviutl2::config::translate(
                "クリックで中間点と継続を切り替え",
            ));

            let color = if matches!(keyframes.keyframes[i], crate::keyframe::Keyframe::Ignored) {
                GUI_COLORS.object_section_ignored
            } else {
                GUI_COLORS.object_section
            };
            painter.rect_filled(rect, 0.0, color);
        }
    }

    fn render_frame_cursor(
        &self,
        painter: &egui::Painter,
        info: &aviutl2::generic::EditInfo,
        object: &SelectedObjectInfo,
        track_rect: egui::Rect,
        view: KeyframeTimelineView,
        total_frames: usize,
    ) {
        if *object.frames.first().unwrap() <= info.frame
            && info.frame <= *object.frames.last().unwrap()
        {
            let position =
                (info.frame - object.frames.first().unwrap()) as f32 / total_frames as f32;
            if position < view.left || view.right < position {
                return;
            }
            let mut rect = track_rect;
            rect.set_left(
                rect.left() + (position - view.left) / view.width() * track_rect.width() - 1.0,
            );
            rect.set_right(rect.left() + 1.0);
            painter.rect_filled(rect, 0.0, GUI_COLORS.frame_cursor);
        }
    }

    fn detach_keyframe_track(
        &self,
        object: &SelectedObjectInfo,
        effect: &EffectInfo,
        params: &crate::KeyframeTrackParams,
        track: &KeyframeTrackInfo,
        name: &str,
    ) {
        let res = crate::EDIT_HANDLE
            .call_edit_section(|edit| {
                let new_params = crate::KeyframeTrackParams::new(edit.info.scene_id);
                if let Some(keyframes) = crate::KEYFRAMES
                    .get(params)
                    .map(|keyframes| keyframes.clone())
                {
                    crate::KEYFRAMES.insert(new_params, keyframes);
                }
                new_params.set_params(edit, effect.handle, name)?;

                // グループ化解除
                if let Some(group_name) = edit
                    .get_effect_track_info(effect.handle, name)?
                    .and_then(|t| t.group_name)
                {
                    edit.set_effect_item_value(effect.handle, &group_name, "0")?;
                }
                anyhow::Ok(())
            })
            .map_err(anyhow::Error::from)
            .flatten();
        match res {
            Ok(()) => {
                tracing::info!(
                    "Detached keyframe track {:?} of effect {:?} in object {:?}",
                    track.names,
                    effect.name,
                    object.name
                );
            }
            Err(e) => {
                tracing::error!(
                    "Failed to detach keyframe track {:?} of effect {:?} in object {:?}: {:?}",
                    track.names,
                    effect.name,
                    object.name,
                    e
                );
            }
        }
    }

    fn show_easing_menu(
        &mut self,
        ui: &mut egui::Ui,
        keyframes: &crate::keyframe::Keyframes,
        params: &crate::KeyframeTrackParams,
        effect: &EffectInfo,
        track: &KeyframeTrackInfo,
        index: usize,
        current_level: &str,
        update_keyframe: impl FnOnce(crate::keyframe::Keyframes),
    ) {
        let easings = crate::EASINGS.read().unwrap();
        let mut update_keyframe_once = Some(update_keyframe);
        let mut update_keyframe = |new_keyframes: crate::keyframe::Keyframes| {
            if let Some(f) = update_keyframe_once.take() {
                f(new_keyframes);
            }
        };

        let (keyframe_index, current_keyframe) = &keyframes
            .keyframes
            .iter()
            .enumerate()
            .take(index + 1)
            .rfind(|(_, k)| matches!(k, crate::keyframe::Keyframe::Easing(_)))
            .expect(
                "少なくとも0フレーム目にはイージングが設定されているはずなので、必ず見つかるはず",
            );
        let keyframe_index = *keyframe_index;
        let crate::keyframe::Keyframe::Easing(current_keyframe) = current_keyframe else {
            unreachable!();
        };
        let current_easing = easings.get(&current_keyframe.easing);

        ui.push_id("midpoint_actions", |ui| {
            Self::show_midpoint_actions(ui, keyframes, index, current_level, &mut update_keyframe);
        });
        ui.push_id("easing_options", |ui| {
            if let Some(current_easing) = current_easing {
                self.show_current_easing_options(
                    ui,
                    keyframes,
                    params,
                    effect,
                    track,
                    keyframe_index,
                    current_keyframe,
                    current_easing,
                    index,
                    &mut update_keyframe,
                );
            }
        });
        ui.menu_button(aviutl2::config::translate("移動方法"), |ui| {
            ui.horizontal(|ui| {
                let height = ui.text_style_height(&egui::TextStyle::Button);
                ui.spacing_mut().item_spacing.x = 4.0;

                ui.add_sized(
                    egui::Vec2::new(ui.available_width() - height - 4.0, height),
                    egui::TextEdit::singleline(&mut self.easing_search_text)
                        .margin(egui::Margin::symmetric(4, 0))
                        .hint_text(aviutl2::config::translate("検索")),
                );
                if ui
                    .add(egui::Button::new("×").min_size(egui::Vec2::splat(height)))
                    .on_hover_text(if self.easing_search_text.is_empty() {
                        aviutl2::config::translate("閉じる")
                    } else {
                        aviutl2::config::translate("検索をクリア")
                    })
                    .clicked()
                {
                    if self.easing_search_text.is_empty() {
                        ui.close();
                    } else {
                        self.easing_search_text.clear();
                    }
                }
            });
            ui.separator();
            // TODO: ちゃんとlabelごとに階層にする
            egui::containers::ScrollArea::vertical().show(ui, |ui| {
                Self::show_easing_choices(
                    ui,
                    keyframes,
                    index,
                    &easings,
                    &self.easing_search_text,
                    &mut update_keyframe,
                );
            });
        });
    }

    fn show_midpoint_actions(
        ui: &mut egui::Ui,
        keyframes: &crate::keyframe::Keyframes,
        index: usize,
        current_level: &str,
        update_keyframe: &mut impl FnMut(crate::keyframe::Keyframes),
    ) {
        if !current_level.is_empty() || index == 0 {
            return;
        }

        if ui
            .add(
                egui::Button::new(aviutl2::config::translate("中間点")).selected(matches!(
                    keyframes.keyframes[index],
                    crate::keyframe::Keyframe::Midpoint
                )),
            )
            .clicked()
        {
            let mut new_keyframes = keyframes.clone();
            new_keyframes.keyframes[index] = crate::keyframe::Keyframe::Midpoint;
            update_keyframe(new_keyframes);
        }
        if ui
            .add(
                egui::Button::new(aviutl2::config::translate("継続")).selected(matches!(
                    keyframes.keyframes[index],
                    crate::keyframe::Keyframe::Ignored
                )),
            )
            .clicked()
        {
            let mut new_keyframes = keyframes.clone();
            new_keyframes.keyframes[index] = crate::keyframe::Keyframe::Ignored;
            update_keyframe(new_keyframes);
        }
        ui.separator();
    }

    fn show_current_easing_options(
        &mut self,
        ui: &mut egui::Ui,
        keyframes: &crate::keyframe::Keyframes,
        params: &crate::KeyframeTrackParams,
        effect: &EffectInfo,
        track: &KeyframeTrackInfo,
        keyframe_index: usize,
        current_keyframe: &crate::keyframe::EasingKeyframeInfo,
        current_easing: &crate::keyframe::Easing,
        index: usize,
        update_keyframe: &mut impl FnMut(crate::keyframe::Keyframes),
    ) {
        let mut has_anything = false;
        if current_easing.has_speed {
            Self::show_speed_options(
                ui,
                keyframes,
                keyframe_index,
                current_keyframe,
                update_keyframe,
            );
            has_anything = true;
        }
        has_anything |= Self::show_param_options(
            ui,
            keyframes,
            params,
            keyframe_index,
            current_keyframe,
            current_easing,
            update_keyframe,
        );
        if current_easing.has_timecontrol {
            if ui.button(aviutl2::config::translate("時間制御")).clicked() {
                self.open_timecontrol_editor(
                    params,
                    effect,
                    track,
                    keyframe_index,
                    current_keyframe,
                );
                ui.close();
                tracing::info!(
                    "Opening time control dialog for section {} of track {:?} in effect {:?}",
                    index,
                    track.names,
                    current_easing.name
                );
            }
            has_anything = true;
        }
        if has_anything {
            ui.separator();
        }
    }

    fn show_param_options(
        ui: &mut egui::Ui,
        keyframes: &crate::keyframe::Keyframes,
        params: &crate::KeyframeTrackParams,
        keyframe_index: usize,
        current_keyframe: &crate::keyframe::EasingKeyframeInfo,
        current_easing: &crate::keyframe::Easing,
        update_keyframe: &mut impl FnMut(crate::keyframe::Keyframes),
    ) -> bool {
        if current_easing.params.is_empty() {
            return false;
        }

        for (param_index, (param_name, default_value)) in current_easing.params.iter().enumerate() {
            let current_value = current_keyframe
                .params
                .get(param_index)
                .copied()
                .unwrap_or(*default_value);
            let id = ui.id().with((
                "easing_param",
                *params,
                keyframe_index,
                param_index,
                param_name,
            ));
            let mut value = ui
                .data(|data| data.get_temp::<String>(id))
                .unwrap_or_else(|| Self::format_easing_param_value(current_value));

            ui.horizontal(|ui| {
                let param_name = crate::utils::get_translated_effect_param_name(
                    &current_easing.name,
                    param_name,
                );
                ui.label(format!("{param_name}："));
                let response = ui.add(
                    egui::TextEdit::singleline(&mut value)
                        .desired_width(80.0)
                        .margin(egui::Margin::symmetric(4, 0))
                        .char_limit(32),
                );

                if response.changed() {
                    ui.data_mut(|data| {
                        data.insert_temp(id, value.clone());
                    });
                }

                if response.lost_focus() {
                    ui.data_mut(|data| {
                        data.remove::<String>(id);
                    });

                    let Ok(value) = value.trim().parse::<f64>() else {
                        return;
                    };
                    if (value - current_value).abs() > f64::EPSILON {
                        let mut new_keyframes = keyframes.clone();
                        Self::set_easing_param_value(
                            &mut new_keyframes,
                            current_easing,
                            keyframe_index,
                            param_index,
                            value,
                        );
                        update_keyframe(new_keyframes);
                    }
                }
            });
        }
        true
    }

    fn format_easing_param_value(value: f64) -> String {
        let formatted = format!("{value:.3}");
        formatted
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }

    fn set_easing_param_value(
        keyframes: &mut crate::keyframe::Keyframes,
        current_easing: &crate::keyframe::Easing,
        keyframe_index: usize,
        param_index: usize,
        value: f64,
    ) {
        let crate::keyframe::Keyframe::Easing(ref mut keyframe) =
            keyframes.keyframes[keyframe_index]
        else {
            unreachable!();
        };
        while keyframe.params.len() <= param_index {
            let default_value = current_easing
                .params
                .values()
                .nth(keyframe.params.len())
                .copied()
                .unwrap_or_default();
            keyframe.params.push(default_value);
        }
        keyframe.params[param_index] = value;
    }

    fn show_speed_options(
        ui: &mut egui::Ui,
        keyframes: &crate::keyframe::Keyframes,
        keyframe_index: usize,
        current_keyframe: &crate::keyframe::EasingKeyframeInfo,
        update_keyframe: &mut impl FnMut(crate::keyframe::Keyframes),
    ) {
        let mut current_acceleration = current_keyframe.acceleration;
        if ui
            .checkbox(
                &mut current_acceleration,
                aviutl2::config::translate("加速"),
            )
            .changed()
        {
            let mut new_keyframes = keyframes.clone();
            let crate::keyframe::Keyframe::Easing(ref mut k) =
                new_keyframes.keyframes[keyframe_index]
            else {
                unreachable!();
            };
            k.acceleration = current_acceleration;
            update_keyframe(new_keyframes);
        }

        let mut current_deceleration = current_keyframe.deceleration;
        if ui
            .checkbox(
                &mut current_deceleration,
                aviutl2::config::translate("減速"),
            )
            .changed()
        {
            let mut new_keyframes = keyframes.clone();
            let crate::keyframe::Keyframe::Easing(ref mut k) =
                new_keyframes.keyframes[keyframe_index]
            else {
                unreachable!();
            };
            k.deceleration = current_deceleration;
            update_keyframe(new_keyframes);
        }
    }

    fn show_easing_choices(
        ui: &mut egui::Ui,
        keyframes: &crate::keyframe::Keyframes,
        index: usize,
        easings: &indexmap::IndexMap<String, crate::keyframe::Easing>,
        search_text: &str,
        update_keyframe: &mut impl FnMut(crate::keyframe::Keyframes),
    ) {
        let search_text = search_text.trim();
        if search_text.is_empty() {
            for easing in easings.values() {
                Self::show_easing_choice(ui, keyframes, index, easing, update_keyframe);
            }
            return;
        }

        let mut matcher = nucleo_matcher::Matcher::new(nucleo_matcher::Config::DEFAULT);
        let pattern = nucleo_matcher::pattern::Pattern::parse(
            search_text,
            nucleo_matcher::pattern::CaseMatching::Ignore,
            nucleo_matcher::pattern::Normalization::Smart,
        );
        let items = easings.values().map(|easing| EasingSearchItem {
            easing,
            text: Self::easing_search_text(easing),
        });

        let mut matches = pattern.match_list(items, &mut matcher);
        matches.sort_by_key(|(item, score)| (std::cmp::Reverse(*score), item.easing.name.clone()));
        if matches.is_empty() {
            ui.label(aviutl2::config::translate("見つかりませんでした"));
            return;
        }
        for (item, _) in matches.into_iter().take(100) {
            Self::show_easing_choice(ui, keyframes, index, item.easing, update_keyframe);
        }
    }

    fn easing_search_text(easing: &crate::keyframe::Easing) -> String {
        if let Some(label) = &easing.label {
            format!("{} {label}", easing.name)
        } else {
            easing.name.clone()
        }
    }

    fn show_easing_choice(
        ui: &mut egui::Ui,
        keyframes: &crate::keyframe::Keyframes,
        index: usize,
        easing: &crate::keyframe::Easing,
        update_keyframe: &mut impl FnMut(crate::keyframe::Keyframes),
    ) {
        if ui
            .add(
                egui::Button::new(crate::utils::get_translated_effect_name(&easing.name)).selected(
                    matches!(
                keyframes.keyframes[index],
                crate::keyframe::Keyframe::Easing(ref k)
                if k.easing == easing.name),
                ),
            )
            .clicked()
        {
            let new_keyframes = Self::keyframes_with_easing(keyframes, index, easing);
            update_keyframe(new_keyframes);
            ui.close();
        }
    }

    fn keyframes_with_easing(
        keyframes: &crate::keyframe::Keyframes,
        index: usize,
        easing: &crate::keyframe::Easing,
    ) -> crate::keyframe::Keyframes {
        let mut new_keyframes = keyframes.clone();
        new_keyframes.keyframes[index] =
            crate::keyframe::Keyframe::Easing(crate::keyframe::EasingKeyframeInfo {
                easing: easing.name.clone(),
                acceleration: easing.default_acceleration,
                deceleration: easing.default_deceleration,
                params: easing.params.values().cloned().collect(),
                timecontrol: crate::keyframe::TimeControl::default(),
            });

        if easing.ignore_midpoints {
            Self::ignore_following_midpoints(&mut new_keyframes, index);
        }
        new_keyframes
    }

    fn ignore_following_midpoints(keyframes: &mut crate::keyframe::Keyframes, index: usize) {
        for i in index + 1..keyframes.keyframes.len() {
            if !matches!(keyframes.keyframes[i], crate::keyframe::Keyframe::Midpoint) {
                break;
            }
            keyframes.keyframes[i] = crate::keyframe::Keyframe::Ignored;
        }
    }
}
