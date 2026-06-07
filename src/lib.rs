use std::str::FromStr;

use anyhow::Context;
use aviutl2_eframe::egui::TextBuffer;

mod gui;
mod keyframe;
mod module;

#[aviutl2::plugin(GenericPlugin)]
struct KeyframesAux2 {
    mod2: aviutl2::generic::SubPlugin<crate::module::KeyframesMod2>,
    gui: aviutl2_eframe::EframeWindow,
}

#[derive(Debug, Clone)]
pub struct CurrentSceneUsageInfo {
    pub bank_ids: std::collections::HashSet<usize>,
    pub params: Vec<KeyframeTrackParams>,
}

pub static EFFECTS: std::sync::LazyLock<dashmap::DashMap<String, aviutl2::generic::Effect>> =
    std::sync::LazyLock::new(dashmap::DashMap::new);
pub static EASINGS: std::sync::LazyLock<
    std::sync::RwLock<indexmap::IndexMap<String, crate::keyframe::Easing>>,
> = std::sync::LazyLock::new(|| std::sync::RwLock::new(indexmap::IndexMap::new()));
pub static EDIT_HANDLE: aviutl2::generic::GlobalEditHandle =
    aviutl2::generic::GlobalEditHandle::new();
pub static OBJECT_ID_TO_HANDLE: std::sync::LazyLock<
    dashmap::DashMap<usize, aviutl2::generic::ObjectHandle>,
> = std::sync::LazyLock::new(dashmap::DashMap::new);
pub static KEYFRAMES: std::sync::LazyLock<
    dashmap::DashMap<KeyframeTrackParams, crate::keyframe::Keyframes>,
> = std::sync::LazyLock::new(dashmap::DashMap::new);
pub static CURRENT_BANK: std::sync::LazyLock<std::sync::Mutex<usize>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(1));
pub static CURRENT_KEYFRAMES_ID: std::sync::LazyLock<std::sync::Mutex<usize>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(0));
pub static CURRENT_SCENE_USAGE: std::sync::LazyLock<std::sync::Mutex<CurrentSceneUsageInfo>> =
    std::sync::LazyLock::new(|| {
        std::sync::Mutex::new(CurrentSceneUsageInfo {
            bank_ids: std::collections::HashSet::new(),
            params: vec![],
        })
    });
pub static PROJECT_SESSION_NONCE: std::sync::LazyLock<std::sync::Mutex<usize>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(new_project_session_nonce()));

fn new_project_session_nonce() -> usize {
    rand::random_range(1..32768)
}

pub fn current_project_session_nonce() -> usize {
    *PROJECT_SESSION_NONCE.lock().unwrap()
}

fn refresh_project_session_nonce() {
    *PROJECT_SESSION_NONCE.lock().unwrap() = new_project_session_nonce();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct KeyframeTrackParams {
    pub bank_id: usize,
    pub keyframes_id: usize,
    pub scene_id: i32,
    pub project_session_nonce: usize,
}

impl KeyframeTrackParams {
    pub fn new(scene_id: i32) -> Self {
        let current_bank_id = CURRENT_BANK.lock().unwrap();
        let mut current_keyframes_id = CURRENT_KEYFRAMES_ID.lock().unwrap();
        let params = Self {
            bank_id: *current_bank_id,
            keyframes_id: *current_keyframes_id,
            scene_id,
            project_session_nonce: current_project_session_nonce(),
        };
        *current_keyframes_id += 1;
        params
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct KeyframeBinding {
    pub object: aviutl2::generic::ObjectHandle,
    pub effect_name: String,
    pub effect_index: usize,
    pub track_name: String,
}

impl KeyframeTrackParams {
    pub fn parse(alias: &str) -> Option<Self> {
        static KEYFRAME_PATTERN: lazy_regex::Lazy<lazy_regex::regex::Regex> = lazy_regex::lazy_regex!(
            r",enhanced_tracks\.aux2,\d+\|(?<bank_id>\d+),(?<keyframes_id>\d+),(?<scene_id>\d+),(?<project_session_nonce>\d+)(?:$|\|)"
        );
        let captures = KEYFRAME_PATTERN.captures(alias)?;
        let bank_id: usize = captures.name("bank_id")?.as_str().parse().ok()?;
        let keyframes_id: usize = captures.name("keyframes_id")?.as_str().parse().ok()?;
        let scene_id: i32 = captures.name("scene_id")?.as_str().parse().ok()?;
        let project_session_nonce: usize = captures
            .name("project_session_nonce")?
            .as_str()
            .parse()
            .ok()?;
        Some(Self {
            bank_id,
            keyframes_id,
            scene_id,
            project_session_nonce,
        })
    }
    pub fn set_params(&self, track: &mut String) -> anyhow::Result<()> {
        static STATIC_VALUE_PATTERN: lazy_regex::Lazy<lazy_regex::regex::Regex> =
            lazy_regex::lazy_regex!(r"^[0-9\.]+$");
        if STATIC_VALUE_PATTERN.is_match(track) {
            track.replace_with(&format!(
                "{},{},enhanced_tracks.aux2,0|{},{},{},{}",
                track,
                track,
                self.bank_id,
                self.keyframes_id,
                self.scene_id,
                self.project_session_nonce
            ));
            return Ok(());
        }
        static KEYFRAME_PATTERN: lazy_regex::Lazy<lazy_regex::regex::Regex> =
            lazy_regex::lazy_regex!(r"(?<easing>[^,]+),(?<flags>\d+)(?<rest>$|\|[^|]*$|\|[^|]*\|)");
        let captures = KEYFRAME_PATTERN
            .captures(track)
            .context("Failed to match keyframe alias pattern")?;
        let flags = captures.name("flags").unwrap();
        let rest = captures.name("rest").unwrap();
        let new_alias = format!(
            "enhanced_tracks.aux2,{}|{},{},{},{}{}",
            flags.as_str(),
            self.bank_id,
            self.keyframes_id,
            self.scene_id,
            self.project_session_nonce,
            if let Some(params) = rest.as_str().strip_prefix('|') {
                if let Some((_old_params, expr)) = params.split_once('|') {
                    format!("|{expr}")
                } else {
                    format!("|{params}")
                }
            } else {
                "".to_string()
            }
        );
        let start = captures.get(0).unwrap().start();
        let end = captures.get(0).unwrap().end();
        track.replace_range(start..end, &new_alias);
        Ok(())
    }
}

impl aviutl2::generic::GenericPlugin for KeyframesAux2 {
    fn new(info: aviutl2::common::AviUtl2Info) -> aviutl2::common::AnyResult<Self> {
        aviutl2::tracing_subscriber::fmt()
            .with_max_level(if cfg!(debug_assertions) {
                tracing::Level::DEBUG
            } else {
                tracing::Level::INFO
            })
            .event_format(aviutl2::logger::AviUtl2Formatter)
            .with_writer(aviutl2::logger::AviUtl2LogWriter)
            .init();
        aviutl2::lprintln!(
            "Config initialized?: {:?}",
            aviutl2::config::app_data_path()
        );
        Ok(Self {
            mod2: aviutl2::generic::SubPlugin::new_script_module(&info)?,
            gui: aviutl2_eframe::EframeWindow::new("enhanced_tracks.aux2", crate::gui::create_gui)?,
        })
    }

    fn plugin_info(&self) -> aviutl2::generic::GenericPluginTable {
        aviutl2::generic::GenericPluginTable {
            name: "enhanced_tracks.aux2".into(),
            information: "".into(),
        }
    }

    fn register(&mut self, registry: &mut aviutl2::generic::HostAppHandle) {
        registry.register_script_module(Some("enhanced_tracks.aux2"), &self.mod2);
        let handle = registry.create_edit_handle();
        let window = handle.get_host_app_window_raw().unwrap();
        match self.gui.handle() {
            Ok(handle) => {
                self.gui.egui_ctx().unwrap().set_pixels_per_point(unsafe {
                    windows::Win32::UI::HiDpi::GetDpiForWindow(windows::Win32::Foundation::HWND(
                        window.hwnd.get() as *mut std::ffi::c_void,
                    )) as f32
                        / 96.0
                });
                let _ = registry.register_window_client("enhanced_tracks.aux2", &handle);
            }
            Err(e) => {
                tracing::error!("Failed to register GUI window: {:?}", e);
            }
        }
        EDIT_HANDLE.init(handle);
    }

    fn on_project_load(&mut self, project: &mut aviutl2::generic::ProjectFile) {
        refresh_project_session_nonce();
        if EFFECTS.is_empty() {
            match load_effects() {
                Ok(_) => {
                    tracing::info!("Effects and easings loaded successfully");
                }
                Err(e) => {
                    tracing::error!("Failed to load effects and easings: {:?}", e);
                }
            }
        }

        let last_bank_id: usize = project.deserialize("last_bank_id").unwrap_or(0);
        {
            let mut current_bank = CURRENT_BANK.lock().unwrap();
            *current_bank = last_bank_id + 1;
        }
        let keyframes: Vec<(KeyframeTrackParams, crate::keyframe::Keyframes)> =
            project.deserialize("keyframes").unwrap_or_default();
        KEYFRAMES.clear();
        for (params, keyframes) in keyframes.into_iter() {
            KEYFRAMES.insert(params, keyframes);
        }
    }

    fn on_project_save(&mut self, project: &mut aviutl2::generic::ProjectFile) {
        project.clear_params();
        project
            .serialize("last_bank_id", &*CURRENT_BANK.lock().unwrap())
            .unwrap();
        let info = EDIT_HANDLE.get_edit_info();
        let _ = EDIT_HANDLE.call_read_section(|read| {
            clear_unused_keyframes(&info, read);
        });
        let keyframes: Vec<(KeyframeTrackParams, crate::keyframe::Keyframes)> = KEYFRAMES
            .iter()
            .map(|entry| (*entry.key(), entry.value().clone()))
            .collect();
        project.serialize("keyframes", &keyframes).unwrap();
    }

    fn on_change_scene(&mut self, edit: &aviutl2::generic::EditSection) {
        {
            let mut current_bank_id = CURRENT_BANK.lock().unwrap();
            *current_bank_id += 1;
        }
        clear_unused_keyframes(&edit.info, edit);
    }

    fn on_clear_cache(&mut self, _edit: &aviutl2::generic::EditSection) {
        match load_easings() {
            Ok(_) => {
                tracing::info!("Easings reloaded successfully");
            }
            Err(e) => {
                tracing::error!("Failed to reload easings: {:?}", e);
            }
        }
    }
}

fn clear_unused_keyframes(info: &aviutl2::generic::EditInfo, read: &aviutl2::generic::ReadSection) {
    let mut used_keyframes = std::collections::HashSet::new();
    for layer_index in 0..=info.layer_max {
        let layer = read.layer(layer_index);
        for (position, object) in layer.objects() {
            if let Err(e) = collect_used_keyframes(read, object, &mut used_keyframes) {
                tracing::error!(
                    "Failed to collect used keyframes for object at position {:?} in layer {:?}: {:?}",
                    position,
                    layer.index,
                    e
                );
            }
        }
    }
    let before_len = KEYFRAMES.len();
    let current_bank_id = *CURRENT_BANK.lock().unwrap();
    KEYFRAMES.retain(|params, _| {
        params.bank_id == current_bank_id
            || params.scene_id != info.scene_id
            || used_keyframes.contains(params)
    });
    tracing::info!("Removed {} unused keyframes", before_len - KEYFRAMES.len());
}

fn load_effects() -> anyhow::Result<()> {
    tracing::info!("Loading effects...");
    let effects = EDIT_HANDLE.get_effects();
    for effect in effects {
        EFFECTS.insert(effect.name.clone(), effect);
    }
    tracing::info!("Loaded {} effects", EFFECTS.len());
    load_easings()
}

fn load_easings() -> anyhow::Result<()> {
    tracing::info!("Loading easings...");
    let mut easings = vec![];
    let standard_easings =
        crate::keyframe::Easing::from_multi_script(None, include_str!("../build/@embedded.tra2"));
    tracing::info!("Loaded standard easings: {}", standard_easings.len());
    easings.extend(standard_easings);

    let bundled_easings = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .join("script.tra2");
    if let Ok(content) = std::fs::read_to_string(&bundled_easings) {
        let bundled_easings =
            crate::keyframe::Easing::from_multi_script(Some(bundled_easings), &content);
        for easing in &bundled_easings {
            tracing::info!("Loaded bundled easing: {}", &easing.name);
        }
        easings.extend(bundled_easings);
    }

    let data_dir = aviutl2::config::app_data_path();
    let script_dir = data_dir.join("Script");
    let mut files = vec![];
    for entry in std::fs::read_dir(script_dir)
        .unwrap_or_else(|_| {
            panic!(
                "Failed to read Script directory in app data path: {:?}",
                data_dir.join("Script")
            )
        })
        .flatten()
    {
        let path = entry.path();
        if path.is_file()
            && (path.extension().and_then(|s| s.to_str()) == Some("tra2")
                || path.extension().and_then(|s| s.to_str()) == Some("tra"))
        {
            files.push((None, path.clone()));
        } else if path.is_dir() {
            let Ok(read_dir) = std::fs::read_dir(&path) else {
                tracing::warn!("Failed to read subdirectory {:?} in Script directory", path);
                continue;
            };
            let dir_name = path
                .file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string());
            for sub_entry in read_dir.flatten() {
                let sub_path = sub_entry.path();
                if sub_path.is_file()
                    && (sub_path.extension().and_then(|s| s.to_str()) == Some("tra2")
                        || sub_path.extension().and_then(|s| s.to_str()) == Some("tra"))
                {
                    files.push((dir_name.clone(), sub_path.clone()));
                }
            }
        }
    }
    tracing::info!(
        "Found {} easing script files in Script directory",
        files.len()
    );
    for (label, file) in files {
        let script_bytes = match std::fs::read(&file) {
            Ok(bytes) => bytes,
            Err(e) => {
                tracing::warn!("Failed to read easing script file {:?}: {:?}", file, e);
                continue;
            }
        };
        let encoded = if file.extension().and_then(|s| s.to_str()) == Some("tra") {
            encoding_rs::SHIFT_JIS.decode(&script_bytes).0.to_string()
        } else {
            match String::from_utf8(script_bytes.clone()) {
                Ok(content) => content,
                Err(e) => {
                    tracing::warn!("Failed to decode easing script file {:?}: {:?}", file, e);
                    continue;
                }
            }
        };

        let file_stem = file.file_stem().unwrap_or_default().to_string_lossy();
        let file_name_starts_with_at = file
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .starts_with('@');
        if file_name_starts_with_at {
            let scripts = crate::keyframe::Easing::from_multi_script_with_bytes(
                Some(file.clone()),
                &encoded,
                &script_bytes,
            );
            for mut script in scripts {
                if file_name_starts_with_at {
                    tracing::info!(
                        "Loaded easing from script file: {}{}",
                        script.name,
                        file_stem
                    );
                    script.name = format!("{}{}", script.name, file_stem);
                } else {
                    tracing::info!("Loaded easing from script file: {}", script.name);
                }
                script.label = script.label.or_else(|| label.clone());
                easings.push(script);
            }
        } else {
            let mut easing = crate::keyframe::Easing::from_script_with_bytes(
                Some(file.clone()),
                &file_stem,
                &encoded,
                &script_bytes,
            );
            tracing::info!("Loaded easing from script file: {}", file_stem);
            easing.label = easing.label.or_else(|| label.clone());
            easings.push(easing);
        }
    }

    easings.retain(|easing| easing.name != "enhanced_tracks.aux2");

    tracing::info!("Total easings loaded: {}", easings.len());

    let config = aviutl2::config::app_data_path().join("aviutl2.ini");
    let table = std::fs::read_to_string(&config)
        .ok()
        .and_then(|content| aviutl2::alias::Table::from_str(&content).ok())
        .unwrap_or_default();

    easings.sort_by_key(|easing| {
        table
            .get_table(&format!("Movement.{}", easing.name))
            .and_then(|t| t.get_value("order"))
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(i32::MAX)
    });

    let mut index_map = indexmap::IndexMap::new();
    for easing in easings {
        if index_map.contains_key(&easing.name) {
            tracing::warn!("Duplicate easing name found: {}. Overwriting.", easing.name);
        }
        index_map.insert(easing.name.clone(), easing);
    }
    *EASINGS.write().unwrap() = index_map;

    Ok(())
}

fn collect_used_keyframes(
    edit: &aviutl2::generic::ReadSection,
    object: aviutl2::generic::ObjectHandle,
    used_keyframes: &mut std::collections::HashSet<KeyframeTrackParams>,
) -> anyhow::Result<()> {
    let alias = edit
        .get_object_alias_parsed(object)
        .context("Failed to get object alias")?;
    let objects = alias
        .get_table("Object")
        .context("Failed to get Object table")?;
    for object in objects.iter_subtables_as_array() {
        let effect_name = object
            .get_value("effect.name")
            .context("Failed to get effect name")?;
        crate::EDIT_HANDLE.enumerate_effect_items(effect_name, |item| {
            if item.item_type != aviutl2::generic::EffectItemType::Number {
                return;
            }
            let Some(value) = object.get_value(&item.name) else {
                return;
            };
            let Some(params) = crate::KeyframeTrackParams::parse(value) else {
                return;
            };
            used_keyframes.insert(params);
        })?;
    }
    Ok(())
}

aviutl2::register_generic_plugin!(KeyframesAux2);

#[cfg(test)]
mod tests {
    use crate::KeyframeTrackParams;

    #[test]
    fn test_keyframe_track_params_parse() {
        let alias = "0,2,enhanced_tracks.aux2,0|1,2,3,4";
        let params = KeyframeTrackParams::parse(alias).unwrap();
        assert_eq!(params.bank_id, 1);
        assert_eq!(params.keyframes_id, 2);
        assert_eq!(params.scene_id, 3);
        assert_eq!(params.project_session_nonce, 4);
    }

    #[test]
    fn test_keyframe_track_params_set_params_static() {
        let mut track = "0.5".to_string();
        let params = KeyframeTrackParams {
            bank_id: 1,
            keyframes_id: 2,
            scene_id: 3,
            project_session_nonce: 4,
        };
        params.set_params(&mut track).unwrap();
        assert_eq!(track, "0.5,0.5,enhanced_tracks.aux2,0|1,2,3,4");
    }

    #[test]
    fn test_keyframe_track_params_set_params_keyframe_no_params() {
        let mut track = "0,0,easeInOutSine,8|test".to_string();
        let params = KeyframeTrackParams {
            bank_id: 1,
            keyframes_id: 2,
            scene_id: 3,
            project_session_nonce: 4,
        };
        params.set_params(&mut track).unwrap();
        assert_eq!(track, "0,0,enhanced_tracks.aux2,8|1,2,3,4|test");
    }

    #[test]
    fn test_keyframe_track_params_set_params_keyframe_with_params() {
        let mut track = "0,0,easeInOutSine,8|1,2|test".to_string();
        let params = KeyframeTrackParams {
            bank_id: 1,
            keyframes_id: 2,
            scene_id: 3,
            project_session_nonce: 4,
        };
        params.set_params(&mut track).unwrap();
        assert_eq!(track, "0,0,enhanced_tracks.aux2,8|1,2,3,4|test");
    }
}
