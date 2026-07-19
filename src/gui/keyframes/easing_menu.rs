use aviutl2_eframe::egui::IntoAtoms as _;

use super::*;

static PINNED_EASINGS_ID: std::sync::LazyLock<egui::Id> =
    std::sync::LazyLock::new(|| egui::Id::new("pinned_easings"));

struct EasingSearchItem<'a> {
    easing: &'a crate::keyframe::Easing,
    text: String,
}

enum EasingChoiceNode<'a> {
    Easing(&'a crate::keyframe::Easing),
    Label {
        label: String,
        children: Vec<EasingChoiceNode<'a>>,
    },
}

impl AsRef<str> for EasingSearchItem<'_> {
    fn as_ref(&self) -> &str {
        &self.text
    }
}

impl KeyframesGui {
    pub(super) fn show_easing_menu(
        &mut self,
        ui: &mut egui::Ui,
        keyframes: &crate::keyframe::Keyframes,
        params: &crate::KeyframeTrackParams,
        object: &SelectedObjectInfo,
        effect: &EffectInfo,
        track: &KeyframeTrackInfo,
        index: usize,
        current_level: &str,
        update_keyframe: impl FnOnce(crate::keyframe::Keyframes),
    ) {
        let mut accesskey = crate::gui::accesskey::AccessKeyContext::root(ui.ctx());
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
        let mut accesskey = accesskey.child();
        ui.push_id("midpoint_actions", |ui| {
            Self::show_midpoint_actions(
                ui,
                &mut accesskey,
                keyframes,
                index,
                current_level,
                &mut update_keyframe,
            );
        });
        ui.push_id("easing_options", |ui| {
            if let Some(current_easing) = current_easing {
                self.show_current_easing_options(
                    &mut accesskey,
                    ui,
                    keyframes,
                    params,
                    object,
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
        let pinned_easings = ui.data_mut(|data| {
            data.get_persisted::<std::sync::Arc<std::sync::Mutex<Vec<String>>>>(*PINNED_EASINGS_ID)
        });
        if let Some(pinned_easings) = pinned_easings {
            let pinned_easings = {
                let pinned_easings = pinned_easings.lock().unwrap();

                pinned_easings
                    .iter()
                    .filter_map(|easing| easings.get(easing))
                    .map(EasingChoiceNode::Easing)
                    .collect::<Vec<_>>()
            };
            self.show_easing_choice_nodes(
                ui,
                &mut accesskey,
                keyframes,
                object,
                effect,
                track,
                index,
                &pinned_easings,
            );
        }
        // ui.menu_button(aviutl2::config::translate("移動方法"), |ui| {
        accesskey.add_menu_button(
            ui,
            egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::E),
            aviutl2::config::translate("移動方法"),
            |ui, accesskey| {
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
                let easing_search_text = self.easing_search_text.clone();
                egui::containers::ScrollArea::vertical().show(ui, |ui| {
                    self.show_easing_choices(
                        ui,
                        accesskey,
                        keyframes,
                        object,
                        effect,
                        track,
                        index,
                        &easings,
                        &easing_search_text,
                    );
                });
            },
        );
    }

    fn show_midpoint_actions(
        ui: &mut egui::Ui,
        accesskey: &mut crate::gui::accesskey::AccessKeyContext,
        keyframes: &crate::keyframe::Keyframes,
        index: usize,
        current_level: &str,
        update_keyframe: &mut impl FnMut(crate::keyframe::Keyframes),
    ) {
        if !current_level.is_empty() || index == 0 {
            return;
        }
        let crate::keyframe::Keyframe::Easing(last_easing) = keyframes
            .keyframes
            .iter()
            .take(index + 1)
            .rfind(|k| matches!(k, crate::keyframe::Keyframe::Easing(_)))
            .expect(
                "少なくとも0フレーム目にはイージングが設定されているはずなので、必ず見つかるはず",
            )
        else {
            unreachable!()
        };
        let easings = crate::EASINGS.read().unwrap();
        let easing_info = easings.get(&last_easing.easing).unwrap_or_default();

        if accesskey
            .add_button_enabled(
                ui,
                !easing_info.ignore_midpoints,
                egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::M),
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
        if accesskey
            .add_button(
                ui,
                egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::C),
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
        accesskey: &mut crate::gui::accesskey::AccessKeyContext,
        ui: &mut egui::Ui,
        keyframes: &crate::keyframe::Keyframes,
        params: &crate::KeyframeTrackParams,
        object: &SelectedObjectInfo,
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
                accesskey,
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
            if accesskey
                .add_button(
                    ui,
                    egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::T),
                    egui::Button::new(aviutl2::config::translate("時間制御")),
                )
                .clicked()
            {
                self.open_timecontrol_editor(
                    params,
                    object,
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
                ui.label(format!("{param_name}: "));
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
        accesskey: &mut crate::gui::accesskey::AccessKeyContext,
        ui: &mut egui::Ui,
        keyframes: &crate::keyframe::Keyframes,
        keyframe_index: usize,
        current_keyframe: &crate::keyframe::EasingKeyframeInfo,
        update_keyframe: &mut impl FnMut(crate::keyframe::Keyframes),
    ) {
        let mut current_acceleration = current_keyframe.acceleration;
        if accesskey
            .add_checkbox(
                ui,
                egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::A),
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
        if accesskey
            .add_checkbox(
                ui,
                egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::D),
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
        &mut self,
        ui: &mut egui::Ui,
        accesskey: &mut crate::gui::accesskey::AccessKeyContext,
        keyframes: &crate::keyframe::Keyframes,
        object: &SelectedObjectInfo,
        effect: &EffectInfo,
        track: &KeyframeTrackInfo,
        index: usize,
        easings: &indexmap::IndexMap<String, crate::keyframe::Easing>,
        search_text: &str,
    ) {
        let search_text = search_text.trim();
        if search_text.is_empty() {
            let choices = Self::build_easing_choice_tree(easings.values());
            self.show_easing_choice_nodes(
                ui, accesskey, keyframes, object, effect, track, index, &choices,
            );
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
            self.show_easing_choice(
                ui,
                accesskey,
                keyframes,
                object,
                effect,
                track,
                index,
                item.easing,
            );
        }
    }

    fn build_easing_choice_tree<'a>(
        easings: impl IntoIterator<Item = &'a crate::keyframe::Easing>,
    ) -> Vec<EasingChoiceNode<'a>> {
        let mut root = Vec::new();
        for easing in easings {
            let Some(label) = &easing.label else {
                root.push(EasingChoiceNode::Easing(easing));
                continue;
            };
            if label.is_empty() {
                root.push(EasingChoiceNode::Easing(easing));
                continue;
            }
            let label = aviutl2::config::get_language_text("Effect", &label);

            let mut children = &mut root;
            for segment in label.split('\\') {
                let existing_index = children.iter().position(|node| {
                    matches!(
                        node,
                        EasingChoiceNode::Label { label, .. } if label == segment
                    )
                });
                let label_index = if let Some(index) = existing_index {
                    index
                } else {
                    children.push(EasingChoiceNode::Label {
                        label: segment.to_string(),
                        children: Vec::new(),
                    });
                    children.len() - 1
                };
                let EasingChoiceNode::Label {
                    children: label_children,
                    ..
                } = &mut children[label_index]
                else {
                    unreachable!();
                };
                children = label_children;
            }
            children.push(EasingChoiceNode::Easing(easing));
        }
        root
    }

    fn show_easing_choice_nodes(
        &mut self,
        ui: &mut egui::Ui,
        accesskey: &mut crate::gui::accesskey::AccessKeyContext,
        keyframes: &crate::keyframe::Keyframes,
        object: &SelectedObjectInfo,
        effect: &EffectInfo,
        track: &KeyframeTrackInfo,
        index: usize,
        nodes: &[EasingChoiceNode<'_>],
    ) {
        for node in nodes {
            match node {
                EasingChoiceNode::Easing(easing) => {
                    self.show_easing_choice(
                        ui, accesskey, keyframes, object, effect, track, index, easing,
                    );
                }
                EasingChoiceNode::Label { label, children } => {
                    let (label, key) = crate::gui::accesskey::parse_accesskey(
                        &crate::utils::get_translated_effect_menu_name(label),
                    );
                    // ui.menu_button(label.as_str(), |ui| {
                    accesskey.add_menu_button(ui, key, label, |ui, accesskey| {
                        egui::containers::ScrollArea::vertical().show(ui, |ui| {
                            self.show_easing_choice_nodes(
                                ui, accesskey, keyframes, object, effect, track, index, children,
                            );
                        });
                    });
                }
            }
        }
    }

    fn easing_search_text(easing: &crate::keyframe::Easing) -> String {
        if let Some(label) = &easing.label {
            format!(
                "{} {}",
                crate::utils::get_translated_effect_name(&easing.name),
                aviutl2::config::get_language_text("Effect", label)
            )
        } else {
            crate::utils::get_translated_effect_name(&easing.name)
        }
    }

    fn show_easing_choice(
        &mut self,
        ui: &mut egui::Ui,
        accesskey: &mut crate::gui::accesskey::AccessKeyContext,
        keyframes: &crate::keyframe::Keyframes,
        object: &SelectedObjectInfo,
        effect: &EffectInfo,
        track: &KeyframeTrackInfo,
        index: usize,
        easing: &crate::keyframe::Easing,
    ) {
        let name = crate::utils::get_translated_effect_menu_name(&easing.name);
        let (name, key) = crate::gui::accesskey::parse_accesskey(&name);
        let button = accesskey.add_button(
            ui,
            key,
            egui::Button::new({
                if easing.has_timecontrol {
                    // NOTE: 左に時計を持ってくると先頭がガタガタして良くないので、右に持ってくる
                    (name, CLOCK.into_atoms()).into_atoms()
                } else {
                    name
                }
            })
            .selected(matches!(
                keyframes.keyframes[index],
                crate::keyframe::Keyframe::Easing(ref k) if k.easing == easing.name
            )),
        );
        Self::show_easing_context_menu(&button, easing);

        if button.clicked() {
            let new_keyframes = Self::keyframes_with_easing(keyframes, index, easing);
            let updated_params =
                Self::update_track_keyframes(effect, track, index, new_keyframes.clone());
            if easing.has_timecontrol {
                let Some(updated_params) = updated_params else {
                    ui.close();
                    return;
                };
                let crate::keyframe::Keyframe::Easing(keyframe) = &new_keyframes.keyframes[index]
                else {
                    unreachable!();
                };
                self.open_timecontrol_editor(
                    &updated_params,
                    object,
                    effect,
                    track,
                    index,
                    keyframe,
                );
                tracing::info!(
                    "Opening time control dialog after selecting easing for section {} of track {:?} in effect {:?}",
                    index,
                    track.names,
                    effect.name
                );
            }
            ui.close();
        }
    }

    fn show_easing_context_menu(button: &egui::Response, easing: &crate::keyframe::Easing) {
        super::sub_context_menu::show(button, |ui| {
            let pinned = ui.data_mut(|data| {
                data.get_persisted::<std::sync::Arc<std::sync::Mutex<Vec<String>>>>(
                    *PINNED_EASINGS_ID,
                )
                .unwrap_or_else(|| {
                    let pinned = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
                    data.insert_persisted(*PINNED_EASINGS_ID, pinned.clone());
                    pinned
                })
            });
            let is_pinned = pinned.lock().unwrap().contains(&easing.name);
            let label = if is_pinned {
                "ピン留めから外す"
            } else {
                "ピン留め"
            };
            if ui
                .button(crate::utils::get_translated_effect_name(label))
                .clicked()
            {
                let mut pinned = pinned.lock().unwrap();
                if is_pinned {
                    pinned.retain(|name| name != &easing.name);
                } else {
                    pinned.push(easing.name.clone());
                }
                ui.close();
            }
        });
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
