use anyhow::Context;
use aviutl2::module::ScriptModuleFunctions;

pub static DEBUG_MODE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[aviutl2::plugin(ScriptModule)]
pub struct KeyframesMod2 {}

impl aviutl2::module::ScriptModule for KeyframesMod2 {
    fn new(_info: aviutl2::common::AviUtl2Info) -> aviutl2::common::AnyResult<Self> {
        Ok(Self {})
    }

    fn plugin_info(&self) -> aviutl2::module::ScriptModuleTable {
        aviutl2::module::ScriptModuleTable {
            information: "enhanced_tracks.aux2: internal module".into(),
            functions: Self::functions(),
        }
    }
}

#[aviutl2::module::functions]
impl KeyframesMod2 {
    #[expect(clippy::type_complexity)]
    fn get_keyframe(
        &self,
        bank_id: i32,
        track_id: i32,
        scene_id: i32,
        project_session_nonce: i32,
        index: usize,
    ) -> aviutl2::common::AnyResult<(
        Vec<i32>,
        String,
        *const u8,
        i32,
        String,
        bool,
        bool,
        Vec<f64>,
    )> {
        let param = crate::KeyframeTrackParams {
            bank_id: bank_id as _,
            keyframes_id: track_id as _,
            scene_id: scene_id as _,
            project_session_nonce: project_session_nonce as _,
        };
        let keyframes = crate::KEYFRAMES.get(&param).with_context(|| {
            format!(
                "keyframes not found for bank_id: {bank_id}, track_id: {track_id}, scene_id: {scene_id}, project_session_nonce: {project_session_nonce}"
            )
        })?;
        let (index, keyframe) = keyframes
            .keyframes
            .iter()
            .enumerate()
            .take(index + 1)
            .rfind(|(_, k)| matches!(k, crate::keyframe::Keyframe::Easing(_)))
            .expect("first keyframe must be easing");
        let mut indices = vec![index as i32];
        let crate::keyframe::Keyframe::Easing(keyframe) = keyframe else {
            unreachable!()
        };
        for i in (index + 1)..keyframes.keyframes.len() {
            match &keyframes.keyframes[i] {
                _ if i == keyframes.keyframes.len() - 1 => {
                    indices.push(i as i32);
                    break;
                }
                crate::keyframe::Keyframe::Easing(_) => {
                    indices.push(i as i32);
                    break;
                }
                crate::keyframe::Keyframe::Midpoint => indices.push(i as i32),
                crate::keyframe::Keyframe::Ignored => (),
            }
        }
        let easings = crate::EASINGS.read().unwrap();
        let easing = easings.get(&keyframe.easing).context("easing not found")?;
        Ok((
            indices,
            easing.name.clone(),
            easing.script_bytes.as_ptr(),
            easing.script_bytes.len().try_into()?,
            easing
                .path
                .clone()
                .unwrap_or_default()
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default(),
            keyframe.acceleration,
            keyframe.deceleration,
            keyframe.params.clone(),
        ))
    }

    fn get_timecontrol_value(
        &self,
        bank_id: i32,
        track_id: i32,
        scene_id: i32,
        project_session_nonce: i32,
        index: usize,
        x: f64,
    ) -> aviutl2::common::AnyResult<f64> {
        let param = crate::KeyframeTrackParams {
            bank_id: bank_id as _,
            keyframes_id: track_id as _,
            scene_id: scene_id as _,
            project_session_nonce: project_session_nonce as _,
        };
        let keyframes = crate::KEYFRAMES
            .get(&param)
            .context("keyframes not found")?;
        let keyframe = keyframes
            .keyframes
            .iter()
            .take(index + 1)
            .rfind(|k| matches!(k, crate::keyframe::Keyframe::Easing(_)))
            .expect("first keyframe must be easing");
        let crate::keyframe::Keyframe::Easing(keyframe) = keyframe else {
            unreachable!()
        };

        Ok(keyframe.timecontrol.y_at_x(x))
    }

    fn debug_mode(&self) -> bool {
        DEBUG_MODE.load(std::sync::atomic::Ordering::Relaxed)
    }
}
