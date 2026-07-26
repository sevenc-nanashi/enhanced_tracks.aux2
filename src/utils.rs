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

pub fn get_translated_effect_menu_name(effect_name: &str) -> String {
    let own_translation = aviutl2::config::get_language_text(effect_name, effect_name);
    if own_translation != effect_name {
        return own_translation;
    }
    let menu_translation = aviutl2::config::get_language_text("Menu", effect_name);
    if menu_translation != effect_name {
        return menu_translation;
    }
    let effect_translation = aviutl2::config::get_language_text("Effect", effect_name);
    if effect_translation != effect_name {
        return effect_translation;
    }
    effect_name.to_string()
}

pub fn get_translated_effect_param_name(effect_name: &str, param_name: &str) -> String {
    let (_, param_name) = param_name.rsplit_once("::").unwrap_or(("", param_name));
    get_translated_effect_item(effect_name, param_name)
}

pub fn get_translated_effect_select_label(effect_name: &str, label: &str) -> String {
    get_translated_effect_item(effect_name, label)
}

fn get_translated_effect_item(effect_name: &str, item: &str) -> String {
    let effect_param_translation = aviutl2::config::get_language_text(effect_name, item);
    if effect_param_translation != item {
        return effect_param_translation;
    }
    let effect_param_translation = aviutl2::config::get_language_text("Effect", item);
    if effect_param_translation != item {
        return effect_param_translation;
    }
    item.to_string()
}
