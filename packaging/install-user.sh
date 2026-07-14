#!/bin/sh
set -eu

project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
binary=${TYPELESS_BINARY:-$project_dir/target/release/typeless-ibus-engine}
data_home=${XDG_DATA_HOME:-$HOME/.local/share}
config_home=${XDG_CONFIG_HOME:-$HOME/.config}
libexec_dir=${TYPELESS_LIBEXEC_DIR:-$HOME/.local/libexec}
component_dir=$data_home/ibus/component
icon_dir=$data_home/icons/hicolor/128x128/apps
dropin_dir=$config_home/systemd/user/org.freedesktop.IBus.session.GNOME.service.d

if [ ! -x "$binary" ]; then
  echo "找不到发布版引擎：$binary" >&2
  echo "请先运行 cargo build --release --locked，或设置 TYPELESS_BINARY。" >&2
  exit 1
fi

install -Dm755 "$binary" "$libexec_dir/typeless-ibus-engine"
install -Dm644 "$project_dir/data/typeless.png" "$icon_dir/typeless.png"
mkdir -p "$component_dir" "$dropin_dir"

sed \
  -e "s|/usr/libexec/typeless-ibus-engine|$libexec_dir/typeless-ibus-engine|g" \
  -e "s|/usr/share/icons/hicolor/128x128/apps/typeless.png|$icon_dir/typeless.png|g" \
  "$project_dir/data/typeless.xml" > "$component_dir/typeless.xml"

sed \
  -e "s|@COMPONENT_DIR@|$component_dir|g" \
  "$project_dir/packaging/ibus-user.conf.in" > "$dropin_dir/typeless.conf"

systemctl --user daemon-reload
if systemctl --user is-active --quiet org.freedesktop.IBus.session.GNOME.service; then
  systemctl --user restart org.freedesktop.IBus.session.GNOME.service
fi

echo "Typeless IBus 已安装到当前用户。"
echo "请在 GNOME 设置的输入源中添加 Typeless Voice。"
