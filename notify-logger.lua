#!/usr/bin/env lua

local home = os.getenv("HOME")
local xdg_data = os.getenv("XDG_DATA_HOME") or (home .. "/.local/share")
local data_dir = xdg_data .. "/notify-history"
os.execute("mkdir -p " .. data_dir)

local log_path = data_dir .. "/notifications.log"

local log_file, err = io.open(log_path, "a")
if not log_file then
    io.stderr:write("Failed to open log file: " .. tostring(err) .. "\n")
    os.exit(1)
end

local cmd = "dbus-monitor \"interface='org.freedesktop.Notifications',member='Notify'\""
local pipe = io.popen(cmd, "r")
if not pipe then
    io.stderr:write("Failed to start dbus-monitor\n")
    os.exit(1)
end

local capturing = false
local string_count = 0
local app_name, summary, body

local function flush_entry()
    if app_name and summary then
        local ts = os.time()

        log_file:write(string.format("%d\t%s\t%s\t%s\n",
            ts,
            app_name,
            summary,
            body
        ))

        log_file:flush()

        print("[" .. os.date("%Y-%m-%d %H:%M:%S", ts) .. "] New notification")
    end

    capturing = false
    string_count = 0
    app_name, summary, body = nil, nil, nil
end

for line in pipe:lines() do
    if line:find("member=Notify") then
        capturing = true
        string_count = 0
        app_name, summary, body = nil, nil, nil
    elseif capturing then
        local s = line:match('string%s+"(.*)"')

        if s and string_count < 4 then
            string_count = string_count + 1
            if string_count == 1 then
                app_name = s
            elseif string_count == 3 then
                summary = s
            elseif string_count == 4 then
                body = s

                flush_entry()
            end
        end
    end
end

pipe:close()
log_file:close()
