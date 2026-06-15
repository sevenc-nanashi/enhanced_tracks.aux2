use anyhow::Context;
use aviutl2_eframe::{eframe, egui};
use tap::prelude::*;

mod keyframes;
mod timecontrol;

pub struct KeyframesGui {
    pub selected_object_info: Option<SelectedObjectInfo>,
    pub timecontrol_editor: Option<TimeControlEditorTarget>,
    pub timecontrol_clipboard: Option<crate::keyframe::TimeControl>,
    pub debug_counter: usize,
}

#[derive(Debug, Clone)]
pub struct TimeControlEditorTarget {
    pub params: crate::KeyframeTrackParams,
    pub keyframe_index: usize,
    pub object: aviutl2::generic::ObjectHandle,
    pub effect_name: String,
    pub effect_index: usize,
    pub track_names: Vec<String>,
    pub timecontrol: crate::keyframe::TimeControl,
    pub selected_point: usize,
    pub context_menu_position: Option<[f64; 2]>,
    pub preset_panel_width: f32,
    pub visible_y_bounds: Option<TimeControlVerticalBounds>,
    pub drag_scroll_y_bounds: Option<TimeControlVerticalBounds>,
    pub dirty: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct TimeControlVerticalBounds {
    pub min_y: f64,
    pub max_y: f64,
}

impl TimeControlVerticalBounds {
    fn y_range(self) -> f64 {
        self.max_y - self.min_y
    }

    fn center(self) -> f64 {
        (self.min_y + self.max_y) / 2.0
    }

    fn translate(self, delta: f64) -> Self {
        Self {
            min_y: self.min_y + delta,
            max_y: self.max_y + delta,
        }
    }

    fn union(self, other: Self) -> Self {
        Self {
            min_y: self.min_y.min(other.min_y),
            max_y: self.max_y.max(other.max_y),
        }
    }

    fn with_center_and_range(center: f64, range: f64) -> Self {
        let half_range = range / 2.0;
        Self {
            min_y: center - half_range,
            max_y: center + half_range,
        }
    }

    fn with_anchor_and_range(anchor_y: f64, anchor_ratio: f64, range: f64) -> Self {
        let min_y = anchor_y - anchor_ratio * range;
        Self {
            min_y,
            max_y: min_y + range,
        }
    }

    fn clamp_to_content(self, content: Self) -> Self {
        let content_range = content.y_range().max(0.000_001);
        let range = self.y_range().clamp(content_range / 8.0, content_range);
        if range >= content_range {
            return content;
        }

        let mut bounds = Self::with_center_and_range(self.center(), range);
        if bounds.min_y < content.min_y {
            bounds = bounds.translate(content.min_y - bounds.min_y);
        }
        if bounds.max_y > content.max_y {
            bounds = bounds.translate(content.max_y - bounds.max_y);
        }
        bounds
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TimeControlHandleKind {
    In,
    Out,
}
impl TimeControlHandleKind {
    fn id(self) -> &'static str {
        match self {
            TimeControlHandleKind::In => "in",
            TimeControlHandleKind::Out => "out",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectType {
    Control,
    VideoInput,
    VideoEffect,
    VideoFilter,
    AudioInput,
    AudioEffect,
    AudioFilter,
}

#[derive(Debug, Clone)]
pub struct SelectedObjectInfo {
    pub handle: aviutl2::generic::ObjectHandle,
    pub name: String,
    pub frames: Vec<usize>,
    pub effects: Vec<EffectInfo>,
}

#[derive(Debug, Clone)]
pub struct EffectInfo {
    pub name: String,
    pub index: usize,
    pub effect_type: EffectType,
    pub keyframe_tracks: indexmap::IndexMap<crate::KeyframeTrackParams, KeyframeTrackInfo>,
}

#[derive(Debug, Clone)]
pub struct KeyframeTrackInfo {
    pub names: Vec<String>,
}

pub struct GuiColors {
    text: egui::Color32,
    log_warn: egui::Color32,
    frame_cursor: egui::Color32,
    grid_line: egui::Color32,
    zoom_gauge: egui::Color32,
    anchor: egui::Color32,
    anchor_line: egui::Color32,
    anchor_hover: egui::Color32,
    anchor_select: egui::Color32,
    object_section: egui::Color32,
    object_section_ignored: egui::Color32,
    object_control: ObjectColors,
    object_video: ObjectColors,
    object_video_effect: ObjectColors,
    object_video_filter: ObjectColors,
    object_audio: ObjectColors,
    object_audio_effect: ObjectColors,
    object_audio_filter: ObjectColors,
}

#[derive(Clone)]
pub struct ObjectColors {
    normal: Vec<egui::Color32>,
    selected: egui::Color32,
}

pub static GUI_COLORS: std::sync::LazyLock<GuiColors> = std::sync::LazyLock::new(GuiColors::load);

impl GuiColors {
    fn load() -> Self {
        Self {
            text: color_code("Text"),
            log_warn: color_code("LogWarn"),
            frame_cursor: color_code("FrameCursor"),
            grid_line: color_code("GridLine"),
            zoom_gauge: color_code("ZoomGauge"),
            anchor: color_code("Anchor"),
            anchor_line: color_code("AnchorLine"),
            anchor_hover: color_code("AnchorHover"),
            anchor_select: color_code("AnchorSelect"),
            object_section: color_code("ObjectSection"),
            object_section_ignored: color_code("Background"),
            object_control: object_colors("ObjectControl", "ObjectControlSelect"),
            object_video: object_colors("ObjectVideo", "ObjectVideoSelect"),
            object_video_effect: object_colors("ObjectVideoEffect", "ObjectVideoEffectSelect"),
            object_video_filter: object_colors("ObjectVideoFilter", "ObjectVideoFilterSelect"),
            object_audio: object_colors("ObjectAudio", "ObjectAudioSelect"),
            object_audio_effect: object_colors("ObjectAudioEffect", "ObjectAudioEffectSelect"),
            object_audio_filter: object_colors("ObjectAudioFilter", "ObjectAudioFilterSelect"),
        }
    }
}

fn color_code(key: &str) -> egui::Color32 {
    aviutl2::config::get_color_code(key)
        .expect("Null文字はない")
        .unwrap_or_else(|| panic!("{key} が style.conf に存在しない"))
        .pipe(|(r, g, b)| egui::Color32::from_rgb(r, g, b))
}

fn color_codes(key: &str) -> Vec<egui::Color32> {
    aviutl2::config::get_all_color_codes(key)
        .unwrap_or_else(|_| panic!("{key} が style.conf に存在しない"))
        .into_iter()
        .map(|(r, g, b)| egui::Color32::from_rgb(r, g, b))
        .collect()
}

fn object_colors(normal: &str, selected: &str) -> ObjectColors {
    let normal = color_codes(normal);
    let normal = if normal.len() == 1 {
        vec![normal[0], normal[0]]
    } else {
        normal
    };
    ObjectColors {
        normal,
        selected: color_code(selected),
    }
}

pub fn get_colors(object_type: &EffectType) -> (Vec<egui::Color32>, egui::Color32) {
    let colors = match object_type {
        EffectType::Control => &GUI_COLORS.object_control,
        EffectType::VideoInput => &GUI_COLORS.object_video,
        EffectType::VideoEffect => &GUI_COLORS.object_video_effect,
        EffectType::VideoFilter => &GUI_COLORS.object_video_filter,
        EffectType::AudioInput => &GUI_COLORS.object_audio,
        EffectType::AudioEffect => &GUI_COLORS.object_audio_effect,
        EffectType::AudioFilter => &GUI_COLORS.object_audio_filter,
    };
    (colors.normal.clone(), colors.selected)
}

pub fn create_gui(
    cc: &aviutl2_eframe::eframe::CreationContext,
    _handle: aviutl2_eframe::AviUtl2EframeHandle,
) -> Result<Box<dyn aviutl2_eframe::eframe::App>, Box<dyn std::error::Error + Send + Sync>> {
    cc.egui_ctx.all_styles_mut(|style| {
        style.visuals = aviutl2_eframe::aviutl2_visuals();
    });
    cc.egui_ctx.set_fonts(aviutl2_eframe::aviutl2_fonts());
    Ok(Box::new(KeyframesGui {
        selected_object_info: None,
        timecontrol_editor: None,
        timecontrol_clipboard: None,
        debug_counter: 0,
    }))
}

impl aviutl2_eframe::eframe::App for KeyframesGui {
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if crate::EDIT_HANDLE.is_ready() {
            if crate::EDIT_HANDLE
                .get_edit_state()
                .is_ok_and(|state| state == aviutl2::generic::EditState::Save)
            {
                return;
            }

            let update_selected_object_info = crate::EDIT_HANDLE
                .call_read_section(|read| {
                    self.update_selected_object_info(read)?;
                    self.update_timecontrol_editor_target(read)?;
                    anyhow::Ok(())
                })
                .map_err(anyhow::Error::from)
                .flatten();
            if let Err(e) = update_selected_object_info {
                tracing::error!("Failed to update selected object info: {:?}", e);
            }
        }
    }
    fn ui(
        &mut self,
        ui: &mut aviutl2_eframe::egui::Ui,
        _frame: &mut aviutl2_eframe::eframe::Frame,
    ) {
        ui.request_repaint_after(std::time::Duration::from_millis(100));
        egui::CentralPanel::default().show_inside(ui, |ui| {
            if crate::EDIT_HANDLE.is_ready() {
                if self.is_undo_mode() {
                    self.render_undo_mode_warning(ui);
                } else if self.timecontrol_editor.is_some() {
                    self.render_timecontrol_editor(ui);
                } else {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        self.render_selected_object_info(ui);
                    });
                }
            } else {
                ui.label("Initializing...");
            }
        });
    }
}

impl KeyframesGui {
    pub fn update_keyframes_for_tracks(
        object: aviutl2::generic::ObjectHandle,
        effect_name: &str,
        effect_index: usize,
        track_names: &[String],
        new_keyframes: crate::keyframe::Keyframes,
    ) -> anyhow::Result<crate::KeyframeTrackParams> {
        crate::EDIT_HANDLE
            .call_edit_section(|edit| {
                let new_params = crate::KeyframeTrackParams::new(edit.info.scene_id);
                crate::KEYFRAMES.insert(new_params, new_keyframes);
                for name in track_names {
                    let mut before =
                        edit.get_object_effect_item(object, effect_name, effect_index, name)?;
                    new_params.set_params(&mut before)?;
                    edit.set_object_effect_item(object, effect_name, effect_index, name, &before)?;
                }
                anyhow::Ok(new_params)
            })
            .map_err(anyhow::Error::from)
            .flatten()
    }

    fn is_undo_mode(&self) -> bool {
        let Some(selected_object_info) = &self.selected_object_info else {
            return false;
        };

        selected_object_info.effects.iter().any(|effect| {
            effect.keyframe_tracks.keys().any(|params| {
                crate::KEYFRAMES.get(params).is_some_and(|keyframes| {
                    keyframes.keyframes.len() != selected_object_info.frames.len()
                })
            })
        })
    }

    fn render_undo_mode_warning(&self, ui: &mut egui::Ui) {
        let (response, painter) =
            ui.allocate_painter(ui.available_size(), aviutl2_eframe::egui::Sense::click());
        let rect = response.rect;

        if response.clicked() {
            let mut resolved_migrations = crate::watcher::RESOLVED_MIGRATIONS.lock().unwrap();
            resolved_migrations.clear();
        }

        let color = GUI_COLORS.log_warn;

        let mut layout = egui::text::LayoutJob::default();
        layout.append(
            "一時停止中\n",
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::proportional(18.0),
                color,
                ..Default::default()
            },
        );
        layout.append(
            "Undoを妨げないために同期を中断しています。クリックで再同期します。",
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::default(),
                color: GUI_COLORS.text,
                ..Default::default()
            },
        );
        layout.wrap = egui::text::TextWrapping::wrap_at_width(rect.width());
        let galley = painter.layout_job(layout);
        painter.galley(
            rect.center().tap_mut(|pos| {
                pos.x -= galley.size().x / 2.0;
                pos.y -= galley.size().y / 2.0;
            }),
            galley,
            color,
        );
    }

    fn update_selected_object_info(
        &mut self,
        read: &aviutl2::generic::ReadSection,
    ) -> aviutl2::common::AnyResult<()> {
        let selected_object = read.get_focused_object()?;
        let Some(selected_object) = selected_object else {
            self.selected_object_info = None;
            return Ok(());
        };
        let alias = read
            .get_object_alias_parsed(selected_object)
            .context("Failed to get object alias")?;
        let objects = alias
            .get_table("Object")
            .context("Failed to get Object table")?;
        let first_effect = objects
            .get_table("0")
            .context("Failed to get first object in Object table")?;
        let first_effect_name = first_effect
            .get_value("effect.name")
            .context("Failed to get effect name")?;
        let first_effect_info = crate::EFFECTS
            .get(first_effect_name)
            .context("Failed to get effect info")?;
        let first_effect_type = Self::determine_effect_type(&first_effect_info, None);
        let mut effects = Vec::new();
        let mut effect_count = std::collections::HashMap::<String, usize>::new();
        for object in objects.iter_subtables_as_array() {
            let effect_name = object
                .get_value("effect.name")
                .context("Failed to get effect name")?;

            let effect_info = crate::EFFECTS
                .get(effect_name)
                .context("Failed to get effect info")?;
            let effect_type = Self::determine_effect_type(&effect_info, Some(first_effect_type));
            let effect_index = effect_count.entry(effect_name.to_string()).or_insert(0);
            *effect_index += 1;
            let effect_index = *effect_index - 1;

            let mut effect_info = EffectInfo {
                name: effect_name.to_string(),
                effect_type,
                index: effect_index,
                keyframe_tracks: indexmap::IndexMap::new(),
            };
            crate::EDIT_HANDLE.enumerate_effect_items(effect_name, |item| {
                if item.item_type != aviutl2::generic::EffectItemType::Number {
                    return;
                }
                // NOTE:
                // エフェクトごとのカウンターとかが面倒なのでEffectItemはitem_typeのチェックでしか使わない
                let Some(value) = object.get_value(&item.name) else {
                    tracing::error!(
                        "Failed to get value for effect item {:?} in effect {:?}",
                        item.name,
                        effect_name
                    );
                    return;
                };
                if let Some(params) = crate::KeyframeTrackParams::parse(value) {
                    // let keyframe_info = KeyframeTrackInfo {
                    //     name: key.to_string(),
                    //     params,
                    // };
                    // effect_info.keyframe_tracks.push(keyframe_info);
                    effect_info
                        .keyframe_tracks
                        .entry(params)
                        .or_insert_with(|| KeyframeTrackInfo { names: Vec::new() })
                        .names
                        .push(item.name.to_string());
                }
            })?;
            effects.push(effect_info);
        }

        let mut object_name = read.get_object_name(selected_object)?.unwrap_or_else(|| {
            effects
                .iter()
                .find(|e| e.effect_type != EffectType::Control)
                .map(|e| e.name.clone())
                .or_else(|| effects.first().map(|e| e.name.clone()))
                .unwrap_or_else(|| "Unknown Object".to_string())
        });
        let output = effects.iter().find(|e| {
            crate::EFFECTS
                .get(&e.name)
                .is_some_and(|e| e.effect_type == aviutl2::generic::EffectType::Output)
        });
        if let Some(output) = output {
            object_name = format!("{} [{}]", object_name, output.name);
        }

        // 出力制御は上に持っていく
        effects.sort_by_key(|e| {
            if crate::EFFECTS
                .get(&e.name)
                .is_some_and(|e| e.effect_type == aviutl2::generic::EffectType::Output)
            {
                -1
            } else {
                1
            }
        });

        let frames = objects
            .get_value("frame")
            .context("Failed to get frame value")?
            .split(',')
            .filter_map(|s| s.parse::<usize>().ok())
            .collect::<Vec<_>>();
        let selected_object_info = SelectedObjectInfo {
            handle: selected_object,
            name: object_name,
            frames,
            effects,
        };
        self.selected_object_info = Some(selected_object_info);

        Ok(())
    }

    fn determine_effect_type(
        effect_info: &aviutl2::generic::Effect,
        first_effect_type: Option<EffectType>,
    ) -> EffectType {
        match effect_info.effect_type {
            aviutl2::generic::EffectType::Filter
                if matches!(first_effect_type, Some(EffectType::Control)) =>
            {
                if effect_info.flag.audio {
                    EffectType::AudioFilter
                } else {
                    EffectType::VideoFilter
                }
            }
            aviutl2::generic::EffectType::Filter if effect_info.flag.audio => {
                EffectType::AudioEffect
            }
            aviutl2::generic::EffectType::Filter => EffectType::VideoEffect,

            aviutl2::generic::EffectType::Input if effect_info.flag.audio => EffectType::AudioInput,
            aviutl2::generic::EffectType::Input => EffectType::VideoInput,
            aviutl2::generic::EffectType::SceneChange => EffectType::Control,
            aviutl2::generic::EffectType::Control => EffectType::Control,
            aviutl2::generic::EffectType::Output => {
                first_effect_type.unwrap_or(EffectType::Control)
            }
        }
    }
}
