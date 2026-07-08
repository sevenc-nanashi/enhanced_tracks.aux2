pub fn get_translated_effect_name(effect_name: &str) -> String {
    let own_translation = aviutl2::config::get_language_text(effect_name, effect_name);
    if own_translation != effect_name {
        return own_translation;
    }
    let effect_translation = aviutl2::config::get_language_text("Effect", effect_name);
    if effect_translation != effect_name {
        return effect_translation;
    }
    effect_name.to_string()
}

pub fn get_translated_effect_param_name(effect_name: &str, param_name: &str) -> String {
    let effect_param_translation = aviutl2::config::get_language_text(effect_name, param_name);
    if effect_param_translation != param_name {
        return effect_param_translation;
    }
    let effect_param_translation = aviutl2::config::get_language_text("Effect", param_name);
    if effect_param_translation != param_name {
        return effect_param_translation;
    }
    param_name.to_string()
}
