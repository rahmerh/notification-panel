#!/usr/bin/env bash

if pgrep -x notify-panel >/dev/null; then
  pkill -x notify-panel
else
  notify-panel &
fi

