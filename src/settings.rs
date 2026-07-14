#[cfg(target_os = "linux")]
mod linux {
    use gtk::prelude::*;
    use std::process::Command;
    use std::rc::Rc;
    use typeless_ibus::audio;
    use typeless_ibus::config::{self, Config, TriggerMode};

    const APPLICATION_ID: &str = "io.github.day253.Typeless.Settings";
    const DEFAULT_DEVICE_ID: &str = "__default__";
    const IBUS_SERVICE: &str = "org.freedesktop.IBus.session.GNOME.service";
    const TRIGGER_KEYS: &[(&str, &str)] = &[
        ("XF86_Fn", "Fn（XF86_Fn）"),
        ("Control_R", "右 Ctrl"),
        ("Control_L", "左 Ctrl"),
        ("F8", "F8"),
        ("F9", "F9"),
        ("F10", "F10"),
        ("Space", "空格"),
    ];

    pub fn run() {
        let application = gtk::Application::builder()
            .application_id(APPLICATION_ID)
            .build();
        application.connect_activate(build_window);
        application.run();
    }

    fn build_window(application: &gtk::Application) {
        if let Some(window) = application.active_window() {
            window.present();
            return;
        }

        let config_path = match config::config_path() {
            Ok(path) => path,
            Err(error) => {
                show_fatal_error(application, &format!("无法确定配置文件路径：{error:#}"));
                return;
            }
        };
        let (current, initial_status) = match Config::load_or_create(&config_path) {
            Ok(config) => (config, String::new()),
            Err(error) => (
                Config::default(),
                format!("现有配置无法读取，已显示默认值：{error:#}"),
            ),
        };

        let window = gtk::ApplicationWindow::builder()
            .application(application)
            .title("Typeless Voice 设置")
            .default_width(560)
            .default_height(480)
            .resizable(false)
            .build();

        let root = gtk::Box::new(gtk::Orientation::Vertical, 18);
        root.set_margin_top(24);
        root.set_margin_bottom(20);
        root.set_margin_start(24);
        root.set_margin_end(24);

        let title = gtk::Label::builder()
            .label("Typeless Voice")
            .halign(gtk::Align::Start)
            .build();
        title.add_css_class("title-1");
        root.append(&title);

        let subtitle = gtk::Label::builder()
            .label("配置原生 IBus 语音输入行为")
            .halign(gtk::Align::Start)
            .build();
        subtitle.add_css_class("dim-label");
        root.append(&subtitle);

        let grid = gtk::Grid::builder()
            .column_spacing(24)
            .row_spacing(16)
            .build();
        grid.set_hexpand(true);

        let trigger = gtk::ComboBoxText::new();
        trigger.set_hexpand(true);
        populate_trigger_keys(&trigger, &current.trigger_key);
        attach_row(&grid, 0, "触发键", &trigger);

        let mode = gtk::ComboBoxText::new();
        mode.append(Some("hold"), "长按：按下开始，松开结束");
        mode.append(Some("toggle"), "切换：按一次开始，再按一次结束");
        mode.set_active_id(Some(match current.trigger_mode {
            TriggerMode::Hold => "hold",
            TriggerMode::Toggle => "toggle",
        }));
        mode.set_hexpand(true);
        attach_row(&grid, 1, "触发方式", &mode);

        let input_device = gtk::ComboBoxText::new();
        input_device.append(Some(DEFAULT_DEVICE_ID), "系统默认麦克风");
        let mut device_status = String::new();
        match audio::input_devices() {
            Ok(devices) => {
                for device in devices {
                    input_device.append(Some(&device.name), &device.name);
                }
            }
            Err(error) => {
                device_status = format!("读取麦克风列表失败：{error:#}");
            }
        }
        if let Some(selected) = current.input_device.as_deref() {
            if !input_device.set_active_id(Some(selected)) {
                input_device.append(Some(selected), &format!("{selected}（当前配置）"));
                input_device.set_active_id(Some(selected));
            }
        } else {
            input_device.set_active_id(Some(DEFAULT_DEVICE_ID));
        }
        input_device.set_hexpand(true);
        attach_row(&grid, 2, "麦克风", &input_device);

        let max_seconds = gtk::SpinButton::with_range(1.0, 600.0, 1.0);
        max_seconds.set_value(current.max_recording_seconds as f64);
        max_seconds.set_hexpand(true);
        attach_row(&grid, 3, "最长录音（秒）", &max_seconds);
        root.append(&grid);

        let hint = gtk::Label::builder()
            .label("如果笔记本固件不向 Linux 上报 Fn，请改用右 Ctrl 或 F8。Esc 可取消当前录音。")
            .halign(gtk::Align::Start)
            .wrap(true)
            .xalign(0.0)
            .build();
        hint.add_css_class("dim-label");
        root.append(&hint);

        let status = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .wrap(true)
            .xalign(0.0)
            .build();
        status.add_css_class("dim-label");
        let combined_status = [initial_status, device_status]
            .into_iter()
            .filter(|message| !message.is_empty())
            .collect::<Vec<_>>()
            .join("；");
        status.set_label(&combined_status);
        root.append(&status);

        let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
        root.append(&separator);

        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        actions.set_halign(gtk::Align::End);
        let reset = gtk::Button::with_label("恢复默认值");
        let close = gtk::Button::with_label("关闭");
        let save = gtk::Button::with_label("保存并应用");
        save.add_css_class("suggested-action");
        actions.append(&reset);
        actions.append(&close);
        actions.append(&save);
        root.append(&actions);

        let trigger_for_reset = trigger.clone();
        let mode_for_reset = mode.clone();
        let input_for_reset = input_device.clone();
        let seconds_for_reset = max_seconds.clone();
        let status_for_reset = status.clone();
        reset.connect_clicked(move |_| {
            let defaults = Config::default();
            trigger_for_reset.set_active_id(Some(&defaults.trigger_key));
            mode_for_reset.set_active_id(Some("hold"));
            input_for_reset.set_active_id(Some(DEFAULT_DEVICE_ID));
            seconds_for_reset.set_value(defaults.max_recording_seconds as f64);
            status_for_reset.set_label("已恢复默认值；点击“保存并应用”后生效。");
        });

        let window_for_close = window.clone();
        close.connect_clicked(move |_| window_for_close.close());

        let path = Rc::new(config_path);
        let trigger_for_save = trigger.clone();
        let mode_for_save = mode.clone();
        let input_for_save = input_device.clone();
        let seconds_for_save = max_seconds.clone();
        let status_for_save = status.clone();
        save.connect_clicked(move |_| {
            let Some(trigger_key) = trigger_for_save.active_id() else {
                status_for_save.set_label("请选择触发键。");
                return;
            };
            let trigger_mode = match mode_for_save.active_id().as_deref() {
                Some("toggle") => TriggerMode::Toggle,
                _ => TriggerMode::Hold,
            };
            let input_device = input_for_save
                .active_id()
                .filter(|id| id.as_str() != DEFAULT_DEVICE_ID)
                .map(|id| id.to_string());
            let config = Config {
                trigger_key: trigger_key.to_string(),
                trigger_mode,
                input_device,
                max_recording_seconds: seconds_for_save.value_as_int() as u64,
            };

            match config.save(path.as_ref()) {
                Ok(()) if restart_ibus() => {
                    status_for_save.set_label("设置已保存，IBus 已重新加载。");
                }
                Ok(()) => {
                    status_for_save.set_label("设置已保存；重新切换输入源或登录后生效。");
                }
                Err(error) => {
                    status_for_save.set_label(&format!("保存失败：{error:#}"));
                }
            }
        });

        window.set_child(Some(&root));
        window.present();
    }

    fn populate_trigger_keys(combo: &gtk::ComboBoxText, selected: &str) {
        for (id, label) in TRIGGER_KEYS {
            combo.append(Some(id), label);
        }
        if !combo.set_active_id(Some(selected)) {
            combo.append(Some(selected), &format!("{selected}（当前配置）"));
            combo.set_active_id(Some(selected));
        }
    }

    fn attach_row(grid: &gtk::Grid, row: i32, label: &str, widget: &impl IsA<gtk::Widget>) {
        let label = gtk::Label::builder()
            .label(label)
            .halign(gtk::Align::Start)
            .valign(gtk::Align::Center)
            .build();
        grid.attach(&label, 0, row, 1, 1);
        grid.attach(widget, 1, row, 1, 1);
    }

    fn restart_ibus() -> bool {
        Command::new("systemctl")
            .args(["--user", "restart", IBUS_SERVICE])
            .status()
            .is_ok_and(|status| status.success())
    }

    fn show_fatal_error(application: &gtk::Application, message: &str) {
        let window = gtk::ApplicationWindow::builder()
            .application(application)
            .title("Typeless Voice 设置")
            .default_width(480)
            .default_height(180)
            .resizable(false)
            .build();
        let label = gtk::Label::builder()
            .label(message)
            .margin_top(24)
            .margin_bottom(24)
            .margin_start(24)
            .margin_end(24)
            .wrap(true)
            .xalign(0.0)
            .build();
        window.set_child(Some(&label));
        window.present();
    }
}

#[cfg(target_os = "linux")]
fn main() {
    linux::run();
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("typeless-ibus-settings 仅支持 Linux");
}
