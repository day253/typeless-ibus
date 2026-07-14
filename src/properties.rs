use crate::config::{Config, TRIGGER_KEY_CHOICES, TriggerMode, trigger_key_label};
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
    let mode_label = match config.trigger_mode {
        TriggerMode::Hold => "长按",
        TriggerMode::Toggle => "切换",
    };
    let mode_items = vec![
        radio_property(
            "typeless.mode.hold",
            "长按：按下开始，松开结束",
            config.trigger_mode == TriggerMode::Hold,
        ),
        radio_property(
            "typeless.mode.toggle",
            "切换：按一次开始，再按一次结束",
            config.trigger_mode == TriggerMode::Toggle,
        ),
    ];

    let trigger_items = TRIGGER_KEY_CHOICES
        .iter()
        .map(|(value, label)| {
            radio_property(
                &format!("{TRIGGER_PREFIX}{value}"),
                label,
                config.trigger_key == *value,
            )
        })
        .collect();

    Value::new(prop_list(vec![
        menu_property(
            "typeless.mode",
            &format!("触发方式：{mode_label}"),
            mode_items,
        ),
        menu_property(
            "typeless.trigger",
            &format!("触发键：{}", trigger_key_label(&config.trigger_key)),
            trigger_items,
        ),
    ]))
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
        .iter()
        .any(|(value, _)| *value == trigger)
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
}
