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
windowrulev2 = noborder, class:^(notify.panel)$
```

The make file installs a simple toggle script which you can execute however you like. I personally use waybar, see my dotfiles for an example.
