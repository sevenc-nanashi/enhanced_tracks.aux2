use anyhow::Context;

pub static RESOLVED_MIGRATIONS: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashSet<crate::KeyframeTrackParams>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashSet::new()));

#[derive(Debug)]
pub enum WatcherMessage {
    ObjectChanged,
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
                tracing::debug!("Watcher thread received message: {:?}", message);
                match message {
                    WatcherMessage::ObjectChanged => on_object_change(),
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

pub fn on_object_change() {
    let update_bindings = crate::EDIT_HANDLE
        .call_read_section(update_keyframe_bindings)
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

fn update_keyframe_bindings(
    read: &aviutl2::generic::ReadSection,
) -> aviutl2::common::AnyResult<
    indexmap::IndexMap<crate::KeyframeBinding, crate::KeyframeTrackParams>,
> {
    let info = crate::EDIT_HANDLE.get_edit_info();
    let mut bindings =
        indexmap::IndexMap::<crate::KeyframeTrackParams, Vec<crate::KeyframeBinding>>::new();

    for layer in 0..=info.layer_max {
        for (_, object) in read.objects_in_layer(layer) {
            collect_object_keyframe_bindings(read, object, &mut bindings)?;
        }
    }

    let mut change_bindings =
        indexmap::IndexMap::<crate::KeyframeBinding, crate::KeyframeTrackParams>::new();
    let mut migrations =
        std::collections::HashMap::<crate::KeyframeTrackParams, crate::KeyframeTrackParams>::new();
    let mut param_to_effect = indexmap::IndexMap::<
        crate::KeyframeTrackParams,
        (aviutl2::generic::ObjectHandle, String, usize),
    >::new();
    let resolved_migrations = RESOLVED_MIGRATIONS.lock().unwrap();
    for (params, bindings) in &bindings {
        for binding in bindings {
            let effect_key = (
                binding.object,
                binding.effect_name.clone(),
                binding.effect_index,
            );
            if resolved_migrations.contains(params) {
                continue;
            }
            if let Some(existing_params) = param_to_effect.get(params)
                && existing_params != &effect_key
            {
                tracing::info!(
                    "Duplicated keyframe track params {:?} for effect {:?} and effect {:?}",
                    params,
                    existing_params,
                    effect_key
                );
                let new_params = *migrations
                    .entry(*params)
                    .or_insert_with(|| crate::KeyframeTrackParams::new(info.scene_id));
                change_bindings.insert(binding.clone(), new_params);
                if let Some(keyframes) = crate::KEYFRAMES
                    .get(params)
                    .map(|keyframes| keyframes.clone())
                {
                    crate::KEYFRAMES.insert(new_params, keyframes);
                }
                migrations.insert(*params, new_params);
            } else if params.bank_id == 0 {
                tracing::info!(
                    "Uninitialized keyframe track params {:?} for effect {:?}",
                    params,
                    effect_key
                );
                let num_sections = read.get_object_section_num(binding.object)?;
                let num_keyframes = num_sections + 1;
                let new_params = crate::KeyframeTrackParams::new(info.scene_id);
                let keyframes = crate::keyframe::Keyframes::new(num_keyframes);
                crate::KEYFRAMES.insert(new_params, keyframes);
                change_bindings.insert(binding.clone(), new_params);
            } else if params.project_session_nonce != crate::current_project_session_nonce() {
                tracing::info!(
                    "Keyframe track params {:?} for effect {:?} has different project session nonce ({} in object, {} in plugin)",
                    params,
                    effect_key,
                    params.project_session_nonce,
                    crate::current_project_session_nonce()
                );
                if let Some(keyframe) = crate::KEYFRAMES.get(params).map(|k| k.clone()) {
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
                    crate::KEYFRAMES.insert(new_params, keyframe);
                    change_bindings.insert(binding.clone(), new_params);
                    migrations.insert(*params, new_params);
                    param_to_effect.insert(*params, effect_key);
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
                    let num_keyframes = read.get_object_section_num(binding.object)? + 1;
                    crate::KEYFRAMES
                        .insert(new_params, crate::keyframe::Keyframes::new(num_keyframes));
                    change_bindings.insert(binding.clone(), new_params);
                    migrations.insert(*params, new_params);
                    param_to_effect.insert(*params, effect_key);
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
                let num_keyframes = read.get_object_section_num(binding.object)? + 1;
                crate::KEYFRAMES.insert(new_params, crate::keyframe::Keyframes::new(num_keyframes));
                change_bindings.insert(binding.clone(), new_params);
                migrations.insert(*params, new_params);
                param_to_effect.insert(*params, effect_key);
            } else {
                let num_keyframes = read.get_object_section_num(binding.object)? + 1;
                match crate::KEYFRAMES.get(params) {
                    None => {
                        tracing::info!(
                            "Keyframe track params {:?} for effect {:?} is not registered in global keyframes map",
                            params,
                            effect_key
                        );
                        crate::KEYFRAMES
                            .insert(*params, crate::keyframe::Keyframes::new(num_keyframes));
                        param_to_effect.insert(*params, effect_key);
                    }
                    Some(existing_keyframes)
                        if existing_keyframes.keyframes.len() != num_keyframes =>
                    {
                        tracing::info!(
                            "Keyframe track params {:?} for effect {:?} has different number of keyframes ({} in global map, {} in object)",
                            params,
                            effect_key,
                            existing_keyframes.keyframes.len(),
                            num_keyframes
                        );
                        let new_params = *migrations
                            .entry(*params)
                            .or_insert_with(|| crate::KeyframeTrackParams::new(info.scene_id));
                        let mut new_keyframes = existing_keyframes.clone();
                        drop(existing_keyframes);
                        new_keyframes.resize(num_keyframes);
                        crate::KEYFRAMES.insert(new_params, new_keyframes);
                        change_bindings.insert(binding.clone(), new_params);
                        param_to_effect.insert(*params, effect_key);
                        migrations.insert(*params, new_params);
                    }
                    Some(_) => {
                        param_to_effect.insert(*params, effect_key);
                    }
                };
            }
        }
    }

    Ok(change_bindings)
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
                    "Updating keyframe track params for object {:?}, effect {:?} (index {}), track {:?} to {:?}",
                    binding.object,
                    binding.effect_name,
                    binding.effect_index,
                    binding.track_name,
                    new_params
                );
                let mut track = edit.get_object_effect_item(
                    binding.object,
                    &binding.effect_name,
                    binding.effect_index,
                    &binding.track_name,
                )?;
                tracing::debug!(
                    "Current keyframe track params for object {:?}, effect {:?} (index {}), track {:?}: {:?}",
                    binding.object,
                    binding.effect_name,
                    binding.effect_index,
                    binding.track_name,
                    &track
                );
                let previous_params = crate::KeyframeTrackParams::parse(&track);
                if let Some(previous_params) = previous_params && previous_params.bank_id != 0 {
                    resolved_migrations.insert(previous_params);
                }
                new_params.set_params(&mut track)?;
                edit.set_object_effect_item(
                    binding.object,
                    &binding.effect_name,
                    binding.effect_index,
                    &binding.track_name,
                    &track,
                )?;
                tracing::debug!(
                    "Updated keyframe track params for object {:?}, effect {:?} (index {}), track {:?} to {:?}",
                    binding.object,
                    binding.effect_name,
                    binding.effect_index,
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
    let alias = read
        .get_object_alias_parsed(object_handle)
        .context("Failed to get object alias")?;
    let objects = alias
        .get_table("Object")
        .context("Failed to get Object table")?;

    let mut effect_count = std::collections::HashMap::<String, usize>::new();
    for object in objects.iter_subtables_as_array() {
        let effect_name = object
            .get_value("effect.name")
            .context("Failed to get effect name")?;
        let effect_index = effect_count.entry(effect_name.to_string()).or_insert(0);
        *effect_index += 1;
        let effect_index = *effect_index - 1;
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
            bindings
                .entry(params)
                .or_default()
                .push(crate::KeyframeBinding {
                    object: object_handle,
                    effect_name: effect_name.to_string(),
                    effect_index,
                    track_name: item.name,
                });
        })?;
    }

    Ok(())
}
