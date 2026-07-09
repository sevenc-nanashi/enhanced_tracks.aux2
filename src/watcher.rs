use anyhow::Context;

type MigrationKey = (
    crate::KeyframeTrackParams,
    aviutl2::generic::ObjectHandle,
    aviutl2::generic::EffectHandle,
);

pub static RESOLVED_MIGRATIONS: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashSet<crate::KeyframeTrackParams>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashSet::new()));
type ResolvedSeedingKey = (aviutl2::generic::EffectHandle, String);
static RESOLVED_SEEDINGS: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashSet<ResolvedSeedingKey>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashSet::new()));

#[derive(Debug)]
pub enum WatcherMessage {
    ObjectChanged,
    FlushResolved,
    Shutdown,
}

pub struct WatcherThread {
    _thread: Option<std::thread::JoinHandle<()>>,
    sender: std::sync::mpsc::Sender<WatcherMessage>,
}

impl WatcherThread {
    pub fn start() -> Self {
        let (sender, receiver) = std::sync::mpsc::channel::<WatcherMessage>();
        let thread = std::thread::spawn(move || {
            tracing::info!("Watcher thread started");
            while let Ok(message) = receiver.recv() {
                tracing::trace!("Watcher thread received message: {:?}", message);
                match message {
                    WatcherMessage::ObjectChanged | WatcherMessage::FlushResolved => {
                        refresh_bindings();

                        if let Some(ctx) = crate::EGUI_CONTEXT.lock().unwrap().as_ref() {
                            ctx.request_repaint();
                        }
                    }
                    WatcherMessage::Shutdown => break,
                }
            }
        });
        Self {
            _thread: Some(thread),
            sender,
        }
    }

    pub fn notify_object_change(&self) {
        if let Err(e) = self.sender.send(WatcherMessage::ObjectChanged) {
            tracing::error!(
                "Failed to send object change message to watcher thread: {:?}",
                e
            );
        }
    }
    pub fn flush_resolved_migrations(&self) {
        let mut resolved_migrations = RESOLVED_MIGRATIONS.lock().unwrap();
        let mut resolved_seedings = RESOLVED_SEEDINGS.lock().unwrap();
        resolved_migrations.clear();
        resolved_seedings.clear();
        if let Err(e) = self.sender.send(WatcherMessage::FlushResolved) {
            tracing::error!(
                "Failed to send continue sync message to watcher thread: {:?}",
                e
            );
        }
    }
}
impl Drop for WatcherThread {
    fn drop(&mut self) {
        if let Err(e) = self.sender.send(WatcherMessage::Shutdown) {
            tracing::error!("Failed to send shutdown message to watcher thread: {:?}", e);
        }
        if let Some(thread) = self._thread.take()
            && let Err(e) = thread.join()
        {
            tracing::error!("Failed to join watcher thread: {:?}", e);
        }
    }
}

pub fn refresh_bindings() {
    let update_bindings = crate::EDIT_HANDLE
        .call_read_section(find_stale_keyframe_bindings)
        .map_err(anyhow::Error::from)
        .flatten();
    let change_bindings = match update_bindings {
        Ok(bindings) => bindings,
        Err(e) => {
            tracing::error!("Failed to update keyframe bindings: {:?}", e);
            return;
        }
    };
    if !change_bindings.is_empty()
        && let Err(e) = apply_bindings_change(change_bindings)
    {
        tracing::error!("Failed to apply keyframe bindings change: {:?}", e);
    }
}

fn find_stale_keyframe_bindings(
    read: &aviutl2::generic::ReadSection,
) -> aviutl2::common::AnyResult<
    indexmap::IndexMap<crate::KeyframeBinding, crate::KeyframeTrackParams>,
> {
    let info = crate::EDIT_HANDLE.get_edit_info();
    let mut bindings =
        indexmap::IndexMap::<crate::KeyframeTrackParams, Vec<crate::KeyframeBinding>>::new();
    tracing::info!(
        "Scanning for keyframe track params in {} layers",
        info.layer_max + 1
    );

    for layer in 0..=info.layer_max {
        for (_, object) in read.objects_in_layer(layer) {
            collect_object_keyframe_bindings(read, object, &mut bindings)?;
        }
    }

    let mut change_bindings =
        indexmap::IndexMap::<crate::KeyframeBinding, crate::KeyframeTrackParams>::new();
    let mut object_migrations =
        std::collections::HashMap::<MigrationKey, crate::KeyframeTrackParams>::new();
    let resolved_migrations = RESOLVED_MIGRATIONS.lock().unwrap();
    for (params, bindings) in &bindings {
        for binding in bindings {
            let effect_key = binding.effect;
            if resolved_migrations.contains(params) {
                continue;
            }
            if params.bank_id == 0 {
                tracing::debug!(
                    "Uninitialized keyframe track params for effect {:?}, skipping",
                    effect_key
                );
                continue;
            } else if params.project_session_nonce != crate::current_project_session_nonce() {
                tracing::info!(
                    "Keyframe track params {:?} for effect {:?} has different project session nonce ({} in object, {} in plugin)",
                    params,
                    effect_key,
                    params.project_session_nonce,
                    crate::current_project_session_nonce()
                );
                if let Some(keyframe) = crate::KEYFRAMES.get(params).map(|k| k.clone()) {
                    let current_frames = object_keyframe_frames(read, binding.object)?;
                    tracing::info!(
                        "Migrating keyframe track params {:?} for effect {:?}",
                        params,
                        effect_key
                    );
                    let new_params = crate::KeyframeTrackParams {
                        project_session_nonce: crate::current_project_session_nonce(),
                        scene_id: info.scene_id,
                        ..*params
                    };
                    let keyframe =
                        remap_keyframes_to_current_frames(params, &keyframe, &current_frames);
                    crate::KEYFRAMES.insert(new_params, keyframe);
                    crate::KEYFRAME_FRAMES.insert(new_params, current_frames);
                    change_bindings.insert(binding.clone(), new_params);
                } else {
                    tracing::warn!(
                        "Keyframe track params {:?} for effect {:?} has different project session nonce but no keyframes found in global map, possibly due to copying from another project session.",
                        params,
                        effect_key
                    );
                    let new_params = crate::KeyframeTrackParams {
                        project_session_nonce: crate::current_project_session_nonce(),
                        scene_id: info.scene_id,
                        ..*params
                    };
                    let current_frames = object_keyframe_frames(read, binding.object)?;
                    crate::KEYFRAMES.insert(
                        new_params,
                        crate::keyframe::Keyframes::new(current_frames.len()),
                    );
                    crate::KEYFRAME_FRAMES.insert(new_params, current_frames);
                    change_bindings.insert(binding.clone(), new_params);
                }
            } else if params.scene_id != info.scene_id {
                tracing::info!(
                    "Keyframe track params {:?} for effect {:?} has different scene id ({} in object, {} in edit info)",
                    params,
                    effect_key,
                    params.scene_id,
                    info.scene_id
                );
                let new_params = crate::KeyframeTrackParams {
                    scene_id: info.scene_id,
                    ..*params
                };
                let current_frames = object_keyframe_frames(read, binding.object)?;
                crate::KEYFRAMES.insert(
                    new_params,
                    crate::keyframe::Keyframes::new(current_frames.len()),
                );
                crate::KEYFRAME_FRAMES.insert(new_params, current_frames);
                change_bindings.insert(binding.clone(), new_params);
            } else {
                let current_frames = object_keyframe_frames(read, binding.object)?;
                match crate::KEYFRAMES.get(params) {
                    None => {
                        tracing::info!(
                            "Keyframe track params {:?} for effect {:?} is not registered in global keyframes map",
                            params,
                            effect_key
                        );
                        crate::KEYFRAMES.insert(
                            *params,
                            crate::keyframe::Keyframes::new(current_frames.len()),
                        );
                        crate::KEYFRAME_FRAMES.insert(*params, current_frames);
                    }
                    Some(existing_keyframes)
                        if existing_keyframes.keyframes.len() != current_frames.len()
                            || keyframe_frames_changed(params, &current_frames) =>
                    {
                        tracing::info!(
                            "Keyframe track params {:?} for effect {:?} has changed frames ({} keyframes in global map, {} frames in object)",
                            params,
                            effect_key,
                            existing_keyframes.keyframes.len(),
                            current_frames.len()
                        );
                        let migration_key = (*params, binding.object, binding.effect);
                        let new_params = *object_migrations
                            .entry(migration_key)
                            .or_insert_with(|| crate::KeyframeTrackParams::new(info.scene_id));
                        let new_keyframes = remap_keyframes_to_current_frames(
                            params,
                            &existing_keyframes,
                            &current_frames,
                        );
                        crate::KEYFRAMES.insert(new_params, new_keyframes);
                        crate::KEYFRAME_FRAMES.insert(new_params, current_frames);
                        change_bindings.insert(binding.clone(), new_params);
                    }
                    Some(_) => {
                        crate::KEYFRAME_FRAMES.insert(*params, current_frames);
                    }
                };
            }
        }
    }

    Ok(change_bindings)
}

fn keyframe_frames_changed(params: &crate::KeyframeTrackParams, current_frames: &[usize]) -> bool {
    crate::KEYFRAME_FRAMES
        .get(params)
        .is_some_and(|previous_frames| previous_frames.as_slice() != current_frames)
}

fn remap_keyframes_to_current_frames(
    params: &crate::KeyframeTrackParams,
    keyframes: &crate::keyframe::Keyframes,
    current_frames: &[usize],
) -> crate::keyframe::Keyframes {
    let Some(previous_frames) = crate::KEYFRAME_FRAMES
        .get(params)
        .map(|frames| frames.clone())
    else {
        let mut resized = keyframes.clone();
        resized.resize(current_frames.len());
        return resized;
    };
    if previous_frames.len() != keyframes.keyframes.len() {
        tracing::warn!(
            "Stored frame count for keyframe track params {:?} does not match keyframe count ({} vs {})",
            params,
            previous_frames.len(),
            keyframes.keyframes.len()
        );
        let mut resized = keyframes.clone();
        resized.resize(current_frames.len());
        return resized;
    }
    keyframes.remap_to_frames(&previous_frames, current_frames)
}

fn object_keyframe_frames(
    read: &aviutl2::generic::ReadSection,
    object: aviutl2::generic::ObjectHandle,
) -> aviutl2::common::AnyResult<Vec<usize>> {
    let section_num = read.get_object_section_num(object)?;
    let mut frames = Vec::with_capacity(section_num + 1);
    for section in 0..section_num {
        frames.push(
            read.get_object_section_frame(object, section)?
                .ok_or_else(|| anyhow::anyhow!("Failed to get frame for section {section}"))?,
        );
    }
    let last_frame = read.get_object_layer_frame(object)?;
    frames.push(last_frame.end + 1);
    Ok(frames)
}

fn apply_bindings_change(
    change_bindings: indexmap::IndexMap<crate::KeyframeBinding, crate::KeyframeTrackParams>,
) -> anyhow::Result<()> {
    tracing::info!(
        "Updating keyframe track params for {} bindings",
        change_bindings.len()
    );
    crate::EDIT_HANDLE
        .call_edit_section(|edit| {
            let mut resolved_migrations = RESOLVED_MIGRATIONS.lock().unwrap();
            for (binding, new_params) in change_bindings {
                tracing::info!(
                    "Updating keyframe track params for object {:?}, effect {:?} ({:?}), track {:?} to {:?}",
                    binding.object,
                    binding.effect_name,
                    binding.effect,
                    binding.track_name,
                    new_params
                );
                let track = edit
                    .effect(binding.effect)
                    .get_item_value(&binding.track_name)?;
                tracing::debug!(
                    "Current keyframe track params for object {:?}, effect {:?} ({:?}), track {:?}: {:?}",
                    binding.object,
                    binding.effect_name,
                    binding.effect,
                    binding.track_name,
                    &track
                );
                let previous_params = crate::KeyframeTrackParams::parse(
                    edit,
                    binding.effect,
                    &binding.track_name,
                );
                if let Some(previous_params) = previous_params && previous_params.bank_id != 0 {
                    resolved_migrations.insert(previous_params);
                }
                new_params.set_params(edit, binding.effect, &binding.track_name)?;
                let track = edit
                    .effect(binding.effect)
                    .get_item_value(&binding.track_name)?;
                tracing::debug!(
                    "Updated keyframe track params for object {:?}, effect {:?} ({:?}), track {:?} to {:?}",
                    binding.object,
                    binding.effect_name,
                    binding.effect,
                    binding.track_name,
                    &track
                );
            }
            anyhow::Ok(())
        })
        .map_err(anyhow::Error::from)
        .flatten()
}

fn collect_object_keyframe_bindings(
    read: &aviutl2::generic::ReadSection,
    object_handle: aviutl2::generic::ObjectHandle,
    bindings: &mut indexmap::IndexMap<crate::KeyframeTrackParams, Vec<crate::KeyframeBinding>>,
) -> aviutl2::common::AnyResult<()> {
    for effect in read.get_effects(object_handle)? {
        let effect = read.effect(effect);
        let effect_name = effect.get_name().context("Failed to get effect name")?;
        crate::EDIT_HANDLE.enumerate_effect_items(&effect_name, |item| {
            if item.item_type != aviutl2::generic::EffectItemType::Number {
                return;
            }
            let Some(params) = crate::KeyframeTrackParams::parse(read, effect.handle, &item.name)
            else {
                return;
            };
            bindings
                .entry(params)
                .or_default()
                .push(crate::KeyframeBinding {
                    object: object_handle,
                    effect: effect.handle,
                    effect_name: effect_name.to_string(),
                    track_name: item.name,
                });
        })?;
    }

    Ok(())
}
