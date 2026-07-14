#!/bin/sh
set -eu

engine=${1:-target/release/typeless-ibus-engine}

if [ ! -x "$engine" ]; then
  echo "IBus protocol test engine is not executable: $engine" >&2
  exit 1
fi

test_root=$(mktemp -d)
engine_log=$test_root/engine.log
engine_pid=
monitor_pid=

cleanup() {
  status=$?
  trap - 0 1 2 15
  if [ -n "$engine_pid" ]; then
    kill "$engine_pid" 2>/dev/null || true
    wait "$engine_pid" 2>/dev/null || true
  fi
  if [ -n "$monitor_pid" ]; then
    kill "$monitor_pid" 2>/dev/null || true
    wait "$monitor_pid" 2>/dev/null || true
  fi
  ibus exit >/dev/null 2>&1 || true
  if [ "$status" -ne 0 ] && [ -f "$engine_log" ]; then
    echo "typeless-ibus engine log:" >&2
    cat "$engine_log" >&2
  fi
  rm -rf "$test_root"
  exit "$status"
}
trap cleanup 0 1 2 15

export HOME=$test_root/home
export XDG_CONFIG_HOME=$HOME/.config
export XDG_DATA_HOME=$HOME/.local/share
export XDG_CACHE_HOME=$HOME/.cache
export DISPLAY=:99
export NO_AT_BRIDGE=1
export LC_ALL=C.UTF-8
mkdir -p "$XDG_CONFIG_HOME" "$XDG_DATA_HOME" "$XDG_CACHE_HOME"

ibus_version_output=$(ibus version)
ibus_version=$(printf '%s\n' "$ibus_version_output" \
  | sed -n 's/^IBus \([0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*\).*$/\1/p')
if [ -z "$ibus_version" ]; then
  echo "Could not parse IBus version from: $ibus_version_output" >&2
  exit 1
fi
version_at_least() (
  current=$1
  minimum=$2
  IFS=.
  set -- $current
  current_major=${1:-0}
  current_minor=${2:-0}
  current_patch=${3:-0}
  set -- $minimum
  minimum_major=${1:-0}
  minimum_minor=${2:-0}
  minimum_patch=${3:-0}

  [ "$current_major" -gt "$minimum_major" ] \
    || { [ "$current_major" -eq "$minimum_major" ] \
      && { [ "$current_minor" -gt "$minimum_minor" ] \
        || { [ "$current_minor" -eq "$minimum_minor" ] \
          && [ "$current_patch" -ge "$minimum_patch" ]; }; }; }
)
if ! version_at_least "$ibus_version" 1.5.22; then
  echo "IBus $ibus_version is older than the supported floor 1.5.22" >&2
  exit 1
fi
echo "Testing IBus protocol against package version $ibus_version"

ibus-daemon --daemonize --replace --xim --panel=disable --config=disable

ibus_address=
attempt=0
while [ "$attempt" -lt 20 ]; do
  ibus_address=$(ibus address 2>/dev/null || true)
  if [ -n "$ibus_address" ] && [ "$ibus_address" != "(null)" ]; then
    break
  fi
  attempt=$((attempt + 1))
  sleep 1
done
if [ -z "$ibus_address" ] || [ "$ibus_address" = "(null)" ]; then
  echo "IBus daemon did not publish an address" >&2
  exit 1
fi
export IBUS_ADDRESS=$ibus_address

"$engine" --ibus >"$engine_log" 2>&1 &
engine_pid=$!

factory_xml=
attempt=0
while [ "$attempt" -lt 20 ]; do
  if factory_xml=$(gdbus introspect \
    --address "$IBUS_ADDRESS" \
    --dest org.freedesktop.IBus.Typeless \
    --object-path /org/freedesktop/IBus/Factory 2>/dev/null); then
    break
  fi
  if ! kill -0 "$engine_pid" 2>/dev/null; then
    echo "typeless-ibus engine exited before registering its Factory" >&2
    exit 1
  fi
  attempt=$((attempt + 1))
  sleep 1
done
printf '%s\n' "$factory_xml" | grep -F 'interface org.freedesktop.IBus.Factory' >/dev/null

create_result=$(gdbus call \
  --address "$IBUS_ADDRESS" \
  --dest org.freedesktop.IBus.Typeless \
  --object-path /org/freedesktop/IBus/Factory \
  --method org.freedesktop.IBus.Factory.CreateEngine \
  typeless)
engine_path=$(printf '%s\n' "$create_result" \
  | sed -n "s|.*'\(/org/freedesktop/IBus/Engine/Typeless/[0-9][0-9]*\)'.*|\1|p")
if [ -z "$engine_path" ]; then
  echo "CreateEngine returned an unexpected value: $create_result" >&2
  exit 1
fi

key_result=$(gdbus call \
  --address "$IBUS_ADDRESS" \
  --dest org.freedesktop.IBus.Typeless \
  --object-path "$engine_path" \
  --method org.freedesktop.IBus.Engine.ProcessKeyEvent \
  97 38 0)
printf '%s\n' "$key_result" | grep -F '(false,)' >/dev/null

property_log=$test_root/properties.log
gdbus monitor \
  --address "$IBUS_ADDRESS" \
  --dest org.freedesktop.IBus.Typeless \
  --object-path "$engine_path" >"$property_log" 2>&1 &
monitor_pid=$!
sleep 1
gdbus call \
  --address "$IBUS_ADDRESS" \
  --dest org.freedesktop.IBus.Typeless \
  --object-path "$engine_path" \
  --method org.freedesktop.IBus.Engine.FocusIn >/dev/null

attempt=0
while [ "$attempt" -lt 10 ]; do
  if grep -F 'Trigger mode:' "$property_log" >/dev/null \
    && grep -F 'Trigger key:' "$property_log" >/dev/null; then
    break
  fi
  attempt=$((attempt + 1))
  sleep 1
done
grep -F 'Trigger mode:' "$property_log" >/dev/null
grep -F 'Trigger key:' "$property_log" >/dev/null

echo "Factory: ok"
echo "CreateEngine: $engine_path"
echo "ProcessKeyEvent: $key_result"
echo "English properties: ok"
