# Notification history panel

Simple notification history panel intended for use with hyprland. Written for personal use, but feel free to use it however you want.

## Installation

There's a makefile provided for easy installation, simply run:

```bash
make install
```

## Usage

There's 2 binaries that work together, the logger writes all notifications to a file and the panel displays them. After installation the logger's systemd service is already installed and enabled, now you only need to run the panel to view them.

It's recommended to use hyprland's window rules to make it behave like a proper side panel, recommended window rules:

```
windowrulev2 = float,      class:^(notify.panel)$
windowrulev2 = size 500 90%, class:^(notify.panel)$
windowrulev2 = move 90% 70, class:^(notify.panel)$
windowrulev2 = nofocus,    class:^(notify.panel)$
windowrulev2 = stayfocused, class:^(notify.panel)$
windowrulev2 = noborder, class:^(notify.panel)$
```

How to toggle the panel is up to you, I personally use waybar and have included a simple bash script in this repo to toggle the panel. See my dotfiles for an example how to use these together.
