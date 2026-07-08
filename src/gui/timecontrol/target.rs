use super::*;

impl KeyframesGui {
    pub fn update_track_keyframes_by_target(
        target: &TimeControlEditorTarget,
        new_keyframes: crate::keyframe::Keyframes,
    ) -> Option<crate::KeyframeTrackParams> {
        tracing::info!(
            "Updating time control keyframe {:?} of track {:?} in effect {:?}",
            target.keyframe_index,
            target.track_names,
            target.effect_name,
        );
        tracing::debug!("New keyframes: {new_keyframes:?}");
        let new_params =
            Self::update_keyframes_for_tracks(target.effect, &target.track_names, new_keyframes);
        match new_params {
            Ok(new_params) => Some(new_params),
            Err(e) => {
                tracing::error!(
                    "Failed to update time control keyframe {:?} of track {:?} in effect {:?}: {:?}",
                    target.keyframe_index,
                    target.track_names,
                    target.effect_name,
                    e
                );
                None
            }
        }
    }

    pub(in crate::gui) fn render_timecontrol_editor(&mut self, ui: &mut egui::Ui) {
        let Some(mut target) = self.timecontrol_editor.clone() else {
            return;
        };
        let easing_name = match crate::KEYFRAMES.get(&target.params).and_then(|keyframes| {
            match keyframes.keyframes.get(target.keyframe_index) {
                Some(crate::keyframe::Keyframe::Easing(kf_info)) => Some(kf_info.easing.clone()),
                _ => None,
            }
        }) {
            Some(easing_name) => easing_name,
            None => {
                self.timecontrol_editor = None;
                return;
            }
        };

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if ui.button(aviutl2::config::translate("戻る")).clicked() {
                self.timecontrol_editor = None;
            }
            ui.label(format!(
                "{}{} / {} / {}",
                aviutl2::config::translate("時間制御："),
                crate::utils::get_translated_effect_name(&target.effect_name),
                if target.track_names.len() == 1 {
                    crate::utils::get_translated_effect_param_name(
                        &target.effect_name,
                        &target.track_names[0],
                    )
                } else {
                    format!(
                        "{} + {}",
                        crate::utils::get_translated_effect_param_name(
                            &target.effect_name,
                            &target.track_names[0]
                        ),
                        target.track_names.len() - 1
                    )
                },
                crate::utils::get_translated_effect_name(&easing_name)
            ));
        });
        ui.separator();

        if self.timecontrol_editor.is_none() {
            return;
        }

        let content_size = ui.available_size();
        let total_width = content_size.x;
        let content_height = content_size.y;
        let separator_width = 8.0;
        if !target.preset_panel_width.is_finite() {
            target.preset_panel_width = (total_width - content_height - separator_width).max(0.0);
        }
        target.preset_panel_width = target
            .preset_panel_width
            .clamp(0.0, (total_width - separator_width).max(0.0));
        let preset_width = target.preset_panel_width;
        let editor_width = (total_width - preset_width - separator_width).max(0.0);
        let (content_rect, _) = ui.allocate_exact_size(content_size, egui::Sense::hover());
        let editor_rect =
            egui::Rect::from_min_size(content_rect.min, egui::vec2(editor_width, content_height));
        let separator_rect = egui::Rect::from_min_size(
            egui::pos2(editor_rect.right(), content_rect.top()),
            egui::vec2(separator_width, content_height),
        );
        let preset_rect = egui::Rect::from_min_size(
            egui::pos2(separator_rect.right(), content_rect.top()),
            egui::vec2(preset_width, content_height),
        );
        let mut result = (false, false);

        if editor_width > 1.0 {
            let mut editor_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(editor_rect)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            editor_ui.set_clip_rect(editor_rect);
            result = Self::show_timecontrol_bezier_editor(
                &mut editor_ui,
                &mut target.timecontrol,
                &mut target.selected_point,
                &mut target.context_menu_position,
                &mut self.timecontrol_clipboard,
                &mut target.visible_y_bounds,
                &mut target.drag_scroll_y_bounds,
            );
        }

        let separator_response = ui.interact(
            separator_rect,
            ui.id().with("timecontrol_editor_separator"),
            egui::Sense::drag(),
        );
        if separator_response.hovered() || separator_response.dragged() {
            ui.output_mut(|output| output.cursor_icon = egui::CursorIcon::ResizeHorizontal);
        }
        ui.painter().line_segment(
            [separator_rect.center_top(), separator_rect.center_bottom()],
            egui::Stroke::new(1.0, GUI_COLORS.grid_line),
        );
        if separator_response.dragged() {
            target.preset_panel_width = (target.preset_panel_width
                - separator_response.drag_delta().x)
                .clamp(0.0, (total_width - separator_width).max(0.0));
        }

        if preset_width > 1.0 {
            let mut preset_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(preset_rect)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            preset_ui.set_clip_rect(preset_rect);
            if let Some(timecontrol) = Self::show_timecontrol_presets(&mut preset_ui) {
                target.timecontrol = timecontrol;
                target.selected_point = 0;
                target.context_menu_position = None;
                target.visible_y_bounds = None;
                target.drag_scroll_y_bounds = None;
                result.0 = true;
                result.1 = true;
            }
        }
        let (changed, commit_requested) = result;
        target.dirty |= changed;

        if commit_requested && target.dirty {
            let Some(mut new_keyframes) = crate::KEYFRAMES
                .get(&target.params)
                .map(|keyframes| keyframes.clone())
            else {
                self.timecontrol_editor = None;
                return;
            };
            let Some(crate::keyframe::Keyframe::Easing(kf_info)) =
                new_keyframes.keyframes.get_mut(target.keyframe_index)
            else {
                self.timecontrol_editor = None;
                return;
            };
            kf_info.timecontrol = target.timecontrol.clone();
            if let Some(new_params) = Self::update_track_keyframes_by_target(&target, new_keyframes)
            {
                target.params = new_params;
                target.dirty = false;
            }
        }

        self.timecontrol_editor = Some(target);
    }
}
