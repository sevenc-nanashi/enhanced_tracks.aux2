use super::*;
use egui::epaint::{Mesh, Shape};

static SECTION_SEPARATOR_HITBOX_WEIGHT: f32 = 4.0;
static KEYFRAME_TIMELINE_FADE_WIDTH: f32 = 24.0;

impl KeyframesGui {
    pub(super) fn render_keyframe_track_info(
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
            object,
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

    pub(super) fn handle_keyframe_timeline_input(&mut self, ui: &egui::Ui, input_rect: egui::Rect) {
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
        object: &SelectedObjectInfo,
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
                self.open_timecontrol_editor(
                    params,
                    object,
                    effect,
                    track,
                    section.0,
                    current_kf_info,
                );
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
                        object,
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

    pub(super) fn open_timecontrol_editor(
        &mut self,
        params: &crate::KeyframeTrackParams,
        object: &SelectedObjectInfo,
        effect: &EffectInfo,
        track: &KeyframeTrackInfo,
        keyframe_index: usize,
        keyframe: &crate::keyframe::EasingKeyframeInfo,
    ) {
        self.timecontrol_editor = Some(TimeControlEditorTarget {
            params: *params,
            keyframe_index,
            object: object.handle,
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
        let easings = crate::EASINGS.read().unwrap();
        if easings.get(&kf_info.easing).is_none() {
            return format!(
                "{}{}",
                crate::utils::get_translated_effect_name(&kf_info.easing),
                aviutl2::config::translate("（不明な移動方法）")
            );
        }
        if kf_info.params.is_empty() {
            return crate::utils::get_translated_effect_name(&kf_info.easing);
        }

        // TODO: ここもi18nするべき？
        format!(
            "{}: {}",
            crate::utils::get_translated_effect_name(&kf_info.easing),
            kf_info
                .params
                .iter()
                .zip(
                    crate::EASINGS
                        .read()
                        .unwrap()
                        .get(&kf_info.easing)
                        .map_or_else(
                            || vec!["?".to_string(); kf_info.params.len()],
                            |easing| easing
                                .params
                                .keys()
                                .map(|k| crate::utils::get_translated_effect_param_name(
                                    &kf_info.easing,
                                    k
                                ))
                                .collect::<Vec<_>>()
                        )
                        .into_iter()
                )
                .map(|(param, name)| format!("{}={}", name, param))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    pub(super) fn update_track_keyframes(
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
        let easings = crate::EASINGS.read().unwrap();
        let mut last_easing_info = match keyframes.keyframes.first() {
            Some(crate::keyframe::Keyframe::Easing(easing)) => easing,
            _ => return,
        };
        for (i, frame) in object.frames.iter().enumerate() {
            last_easing_info = match keyframes.keyframes[i] {
                crate::keyframe::Keyframe::Easing(ref easing) => easing,
                _ => last_easing_info,
            };
            if i == object.frames.len() - 1 {
                continue;
            }
            let last_easing = easings.get(&last_easing_info.easing);
            let easing_label = match keyframes.keyframes[i] {
                crate::keyframe::Keyframe::Easing(ref easing) => {
                    if last_easing.is_some_and(|easing| easing.has_timecontrol) {
                        format!(
                            "{} {}",
                            CLOCK,
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
                &easing_label,
                0.0,
                egui::TextFormat {
                    font_id: egui::FontId::default(),
                    color: if last_easing.is_some() {
                        GUI_COLORS.text
                    } else {
                        GUI_COLORS.log_warn
                    },
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
        let mut first_easing = match keyframes.keyframes.first() {
            Some(crate::keyframe::Keyframe::Easing(easing)) => easing,
            _ => return,
        };
        for (i, frame) in object.frames.iter().enumerate() {
            first_easing = match keyframes.keyframes[i] {
                crate::keyframe::Keyframe::Easing(ref easing) => easing,
                _ => first_easing,
            };

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
            if crate::EASINGS
                .read()
                .unwrap()
                .get(&first_easing.easing)
                .is_some_and(|easing| easing.ignore_midpoints)
            {
                let click = ui.interact(
                    click_rect,
                    ui.id().with("separator").with(i),
                    aviutl2_eframe::egui::Sense::click(),
                );
                click.on_hover_text(aviutl2::config::translate(
                    "この移動方法では中間点の切り替えはできません。",
                ));
            } else {
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
            }

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
}
