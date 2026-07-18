#![allow(clippy::too_many_arguments)]
use super::*;
use aviutl2_eframe::egui;

mod easing_menu;
mod timeline;

static CLOCK: &str = "🕒";

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
        egui::containers::CollapsingHeader::new(if effect.is_output {
            match effect.effect_type {
                EffectType::VideoInput | EffectType::VideoEffect | EffectType::VideoFilter => {
                    format!(
                        "{} [{}]",
                        aviutl2::config::get_language_text("Effect", "描画"),
                        crate::utils::get_translated_effect_name(&effect.name)
                    )
                }
                EffectType::AudioInput | EffectType::AudioEffect | EffectType::AudioFilter => {
                    format!(
                        "{} [{}]",
                        aviutl2::config::get_language_text("Effect", "音声"),
                        crate::utils::get_translated_effect_name(&effect.name)
                    )
                }
                _ => crate::utils::get_translated_effect_name(&effect.name),
            }
        } else {
            crate::utils::get_translated_effect_name(&effect.name)
        })
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
}
