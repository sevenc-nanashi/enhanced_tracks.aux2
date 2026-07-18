use super::*;
impl KeyframesGui {
    pub fn render_debug_view(&mut self, ui: &mut egui::Ui) {
        egui::containers::Window::new("Debug View")
            .open(&mut self.debug_view)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    if ui.button("Print Keyframes").clicked() {
                        tracing::info!("Keyframes: {:#?}", crate::KEYFRAMES);
                    }
                    if ui.button("Print TimeControlEditorTarget").clicked() {
                        if let Some(target) = &self.timecontrol_editor {
                            tracing::info!("TimeControlEditorTarget: {:#?}", target);
                        } else {
                            tracing::info!("TimeControlEditorTarget: None");
                        }
                    }
                    if ui.button("Print Params to Effect Binding").clicked() {
                        let res = print_binding();
                        match res {
                            Ok(_) => {
                                tracing::info!("Params to Effect Binding printed successfully.")
                            }
                            Err(e) => {
                                tracing::error!("Failed to print Params to Effect Binding: {:?}", e)
                            }
                        }
                    }
                    let mut debug_logging =
                        crate::module::DEBUG_MODE.load(std::sync::atomic::Ordering::Relaxed);
                    if ui
                        .checkbox(&mut debug_logging, "Enable Debug Logging")
                        .clicked()
                    {
                        crate::module::DEBUG_MODE
                            .store(debug_logging, std::sync::atomic::Ordering::Relaxed);
                        tracing::info!("Debug logging set to: {}", debug_logging);
                    }
                });
            });
    }
}
fn print_binding() -> anyhow::Result<()> {
    crate::EDIT_HANDLE.call_read_section(|read| {
        let info = crate::EDIT_HANDLE.get_edit_info();
        for layer in 0..=info.layer_max {
            for (pos, object) in read.objects_in_layer(layer) {
                for effect in read.get_effects(object)? {
                    let effect = read.effect(effect);
                    let effect_name = effect.get_name().context("Failed to get effect name")?;
                    crate::EDIT_HANDLE.enumerate_effect_items(&effect_name, |item| {
                        if item.item_type != aviutl2::generic::EffectItemType::Number {
                            return;
                        }
                        let Some(params) =
                            crate::KeyframeTrackParams::parse(read, effect.handle, &item.name)
                        else {
                            return;
                        };

                        tracing::info!(
                            "Params {:?} -> Object {:?}, Effect: {:?}, Item: {:?}",
                            params,
                            pos,
                            effect_name,
                            item.name,
                        );
                    })?;
                }
            }
        }

        Ok(())
    })?
}
