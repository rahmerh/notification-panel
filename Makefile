CARGO ?= cargo

PREFIX ?= $(HOME)/.local
BIN_DIR := $(PREFIX)/bin

SYSTEMD_USER_DIR := $(HOME)/.config/systemd/user
SERVICE_NAME := notify-logger.service

LOGGER_PKG := notify-logger
PANEL_PKG := notify-panel

LOGGER_BIN := notify-logger
PANEL_BIN := notify-panel

SCRIPT_NAME := toggle-notification-panel
SCRIPT_DST := $(BIN_DIR)/$(SCRIPT_NAME)

build-logger:
	$(CARGO) build --release --manifest-path $(LOGGER_PKG)/Cargo.toml

build-panel:
	$(CARGO) build --release --manifest-path $(PANEL_PKG)/Cargo.toml

install-logger: build-logger
	mkdir -p $(BIN_DIR)
	install -Dm755 $(LOGGER_PKG)/target/release/$(LOGGER_BIN) $(BIN_DIR)/$(LOGGER_BIN)

install-panel: build-panel
	mkdir -p $(BIN_DIR)
	install -Dm755 $(PANEL_PKG)/target/release/$(PANEL_BIN) $(BIN_DIR)/$(PANEL_BIN)
	install -Dm755 notify-toggle/$(SCRIPT_NAME) $(SCRIPT_DST)

service: install-logger
	mkdir -p $(SYSTEMD_USER_DIR)
	@echo "[Unit]"                                   >  "$(SYSTEMD_USER_DIR)/$(SERVICE_NAME)"
	@echo "Description=Notification history logger (Rust)" >> "$(SYSTEMD_USER_DIR)/$(SERVICE_NAME)"
	@echo ""                                          >> "$(SYSTEMD_USER_DIR)/$(SERVICE_NAME)"
	@echo "[Service]"                                >> "$(SYSTEMD_USER_DIR)/$(SERVICE_NAME)"
	@echo "ExecStart=$(BIN_DIR)/$(LOGGER_BIN)"          >> "$(SYSTEMD_USER_DIR)/$(SERVICE_NAME)"
	@echo "Restart=on-failure"                       >> "$(SYSTEMD_USER_DIR)/$(SERVICE_NAME)"
	@echo ""                                          >> "$(SYSTEMD_USER_DIR)/$(SERVICE_NAME)"
	@echo "[Install]"                                >> "$(SYSTEMD_USER_DIR)/$(SERVICE_NAME)"
	@echo "WantedBy=default.target"                  >> "$(SYSTEMD_USER_DIR)/$(SERVICE_NAME)"

enable-logger: service
	systemctl --user daemon-reload
	systemctl --user enable --now notify-logger.service

install: install-logger install-panel service enable-logger

uninstall-logger:
	- systemctl --user disable --now notify-logger.service 2>/dev/null || true
	- rm -f $(SYSTEMD_USER_DIR)/notify-logger.service
	- rm -f $(BIN_DIR)/$(LOGGER_BIN)

uninstall-panel:
	- rm -f $(BIN_DIR)/$(PANEL_BIN)

uninstall: uninstall-logger uninstall-panel
