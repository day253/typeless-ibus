use crate::config::{Config, TRIGGER_KEY_CHOICES, TriggerMode};
use crate::i18n::{Language, current_language};
use std::collections::HashMap;
use zbus::zvariant::{Structure, Value};

const PROP_TYPE_RADIO: u32 = 2;
const PROP_TYPE_MENU: u32 = 3;
const PROP_STATE_UNCHECKED: u32 = 0;
const PROP_STATE_CHECKED: u32 = 1;

const MODE_PREFIX: &str = "typeless.mode.";
const TRIGGER_PREFIX: &str = "typeless.trigger.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigAction {
    SetMode(TriggerMode),
    SetTrigger(String),
}

pub fn config_properties(config: &Config) -> Value<'static> {
    config_properties_for_language(config, current_language())
}

struct UiLabels {
    mode: &'static str,
    mode_hold: &'static str,
    mode_toggle: &'static str,
    hold_help: &'static str,
    toggle_help: &'static str,
    trigger: &'static str,
    control_right: &'static str,
    control_left: &'static str,
    space: &'static str,
    separator: &'static str,
}

const ENGLISH_LABELS: UiLabels = UiLabels {
    mode: "Trigger mode",
    mode_hold: "Hold",
    mode_toggle: "Toggle",
    hold_help: "Hold: press to start, release to stop",
    toggle_help: "Toggle: press once to start, again to stop",
    trigger: "Trigger key",
    control_right: "Right Ctrl",
    control_left: "Left Ctrl",
    space: "Space",
    separator: ": ",
};

const CHINESE_LABELS: UiLabels = UiLabels {
    mode: "触发方式",
    mode_hold: "长按",
    mode_toggle: "切换",
    hold_help: "长按：按下开始，松开结束",
    toggle_help: "切换：按一次开始，再按一次结束",
    trigger: "触发键",
    control_right: "右 Ctrl",
    control_left: "左 Ctrl",
    space: "空格",
    separator: "：",
};

fn config_properties_for_language(config: &Config, language: Language) -> Value<'static> {
    let labels = labels(language);
    let mode_label = match config.trigger_mode {
        TriggerMode::Hold => labels.mode_hold,
        TriggerMode::Toggle => labels.mode_toggle,
    };
    let mode_items = vec![
        radio_property(
            "typeless.mode.hold",
            labels.hold_help,
            config.trigger_mode == TriggerMode::Hold,
        ),
        radio_property(
            "typeless.mode.toggle",
            labels.toggle_help,
            config.trigger_mode == TriggerMode::Toggle,
        ),
    ];

    let trigger_items = TRIGGER_KEY_CHOICES
        .iter()
        .map(|value| {
            radio_property(
                &format!("{TRIGGER_PREFIX}{value}"),
                trigger_key_label(labels, value),
                config.trigger_key == *value,
            )
        })
        .collect();

    Value::new(prop_list(vec![
        menu_property(
            "typeless.mode",
            &format!("{}{}{}", labels.mode, labels.separator, mode_label),
            mode_items,
        ),
        menu_property(
            "typeless.trigger",
            &format!(
                "{}{}{}",
                labels.trigger,
                labels.separator,
                trigger_key_label(labels, &config.trigger_key)
            ),
            trigger_items,
        ),
    ]))
}

fn labels(language: Language) -> &'static UiLabels {
    match language {
        Language::English => &ENGLISH_LABELS,
        Language::Chinese => &CHINESE_LABELS,
    }
}

fn trigger_key_label<'a>(labels: &UiLabels, value: &'a str) -> &'a str {
    match value {
        "XF86_Fn" => "Fn",
        "Control_R" => labels.control_right,
        "Control_L" => labels.control_left,
        "Space" => labels.space,
        _ => value,
    }
}

pub fn action_for_activation(name: &str, state: u32) -> Option<ConfigAction> {
    if state != PROP_STATE_CHECKED {
        return None;
    }
    if let Some(mode) = name.strip_prefix(MODE_PREFIX) {
        return match mode {
            "hold" => Some(ConfigAction::SetMode(TriggerMode::Hold)),
            "toggle" => Some(ConfigAction::SetMode(TriggerMode::Toggle)),
            _ => None,
        };
    }
    let trigger = name.strip_prefix(TRIGGER_PREFIX)?;
    TRIGGER_KEY_CHOICES
        .contains(&trigger)
        .then(|| ConfigAction::SetTrigger(trigger.to_string()))
}

fn radio_property(key: &str, label: &str, checked: bool) -> Structure<'static> {
    property(
        key,
        PROP_TYPE_RADIO,
        label,
        if checked {
            PROP_STATE_CHECKED
        } else {
            PROP_STATE_UNCHECKED
        },
        Vec::new(),
    )
}

fn menu_property(key: &str, label: &str, children: Vec<Structure<'static>>) -> Structure<'static> {
    property(key, PROP_TYPE_MENU, label, PROP_STATE_UNCHECKED, children)
}

fn property(
    key: &str,
    property_type: u32,
    label: &str,
    state: u32,
    children: Vec<Structure<'static>>,
) -> Structure<'static> {
    Structure::from((
        "IBusProperty",
        HashMap::<String, Value<'static>>::new(),
        key.to_string(),
        property_type,
        Value::new(ibus_text(label)),
        String::new(),
        Value::new(ibus_text("")),
        true,
        true,
        state,
        Value::new(prop_list(children)),
        Value::new(ibus_text("")),
    ))
}

fn prop_list(properties: Vec<Structure<'static>>) -> Structure<'static> {
    let properties = properties.into_iter().map(Value::new).collect::<Vec<_>>();
    Structure::from((
        "IBusPropList",
        HashMap::<String, Value<'static>>::new(),
        properties,
    ))
}

fn ibus_text(text: &str) -> Structure<'static> {
    let attributes = Structure::from((
        "IBusAttrList",
        HashMap::<String, Value<'static>>::new(),
        Vec::<Value<'static>>::new(),
    ));
    Structure::from((
        "IBusText",
        HashMap::<String, Value<'static>>::new(),
        text.to_string(),
        Value::new(attributes),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn property_list_uses_ibus_serializable_signature() {
        let properties = config_properties(&Config::default());
        assert_eq!(properties.value_signature().to_string(), "(sa{sv}av)");
    }

    #[test]
    fn checked_radio_activations_map_to_config_changes() {
        assert_eq!(
            action_for_activation("typeless.mode.toggle", PROP_STATE_CHECKED),
            Some(ConfigAction::SetMode(TriggerMode::Toggle))
        );
        assert_eq!(
            action_for_activation("typeless.trigger.Control_R", PROP_STATE_CHECKED),
            Some(ConfigAction::SetTrigger("Control_R".to_string()))
        );
        assert_eq!(
            action_for_activation("typeless.mode.hold", PROP_STATE_UNCHECKED),
            None
        );
        assert_eq!(
            action_for_activation("typeless.trigger.Unknown", PROP_STATE_CHECKED),
            None
        );
    }

    #[test]
    fn english_and_chinese_labels_are_complete() {
        assert_eq!(labels(Language::English).mode, "Trigger mode");
        assert_eq!(
            trigger_key_label(labels(Language::English), "Control_R"),
            "Right Ctrl"
        );
        assert_eq!(labels(Language::Chinese).mode, "触发方式");
        assert_eq!(
            trigger_key_label(labels(Language::Chinese), "Control_R"),
            "右 Ctrl"
        );
    }
}
