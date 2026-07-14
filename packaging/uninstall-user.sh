#!/bin/sh
set -eu

data_home=${XDG_DATA_HOME:-$HOME/.local/share}
config_home=${XDG_CONFIG_HOME:-$HOME/.config}
libexec_dir=${TYPELESS_LIBEXEC_DIR:-$HOME/.local/libexec}

rm -f "$libexec_dir/typeless-ibus-engine"
rm -f "$data_home/ibus/component/typeless.xml"
rm -f "$data_home/icons/hicolor/128x128/apps/typeless.png"
rm -f "$config_home/systemd/user/org.freedesktop.IBus.session.GNOME.service.d/typeless.conf"

systemctl --user daemon-reload
if systemctl --user is-active --quiet org.freedesktop.IBus.session.GNOME.service; then
  systemctl --user restart org.freedesktop.IBus.session.GNOME.service
fi

echo "Typeless IBus 用户级安装已移除。"
