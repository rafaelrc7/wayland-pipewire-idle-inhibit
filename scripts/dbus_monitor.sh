#!/usr/bin/env bash
###############################################################################
# Waybar Custom Module: Wayland-Pipewire-Idle-Inhibit Monitor
#
# DESCRIPTION:
#   This script monitors the state of the wayland-pipewire-idle-inhibit service
#   via D-Bus and outputs a JSON object compatible with Waybar's custom module.
#   It tracks three distinct states:
#     1. 'off': The inhibitor service is not running on the session bus.
#     2. 'idle': Service is running, but idle inhibition is currently inactive.
#     3. 'inhibited': Inhibition is active, distinguishing between 'Audio' 
#        (automated) and 'Manual' (user-forced) triggers.
#
# DEPENDENCIES:
#   - jq: For parsing JSON payloads from busctl.
#   - systemd (busctl): To interface with the D-Bus session bus.
#
# WAYBAR CONFIGURATION EXAMPLE:
#   "custom/idle-inhibit": {
#       "return-type": "json",
#       "format": "{icon}",
#       "exec": "/path/to/this/script.sh",
#       "format-icons": {
#           "inhibited": "󰈈",
#           "idle": "󰈉",
#           "off": ""
#       },
#       "on-click": "busctl --user call com.rafaelrc.WaylandPipewireIdleInhibit /com/rafaelrc/WaylandPipewireIdleInhibit com.rafaelrc.WaylandPipewireIdleInhibit ToggleManualInhibit"
#   }
###############################################################################

# Configuration
SERVICE="com.rafaelrc.WaylandPipewireIdleInhibit"
OBJECT="/com/rafaelrc/WaylandPipewireIdleInhibit"
INTERFACE="com.rafaelrc.WaylandPipewireIdleInhibit"

# Function to format and output JSON for Waybar
print_status() {
    local idle_inhibited=$1
    local manual_inhibited=$2

    local tooltip=""
    local class=""
    local alt=""

    # Output if the inhibitor is not running
    if [ "$idle_inhibited" == "off" ]; then
        echo '{"alt": "off", "tooltip": "wayland-pipewire-idle-inhibit not running", "class": "off"}'
        return
    fi

    if [ "$idle_inhibited" == "true" ]; then
        alt="inhibited"
        if [ "$manual_inhibited" == "true" ]; then
            tooltip="Idle Inhibitor: Active (Manual)"
            class="manual"
        else
            tooltip="Idle Inhibitor: Active (Audio)"
            class="inhibited"
        fi
    else
        alt="idle"
        tooltip="Idle Inhibitor: Inactive"
        class="idle"
    fi

    # Output compressed JSON line
    printf '{"alt": "%s", "tooltip": "%s", "class": "%s"}\n' "$alt" "$tooltip" "$class"
}

# Get Initial State
if busctl --user status "$SERVICE" &>/dev/null; then
    # Properties are PascalCase by default in zbus
    IS_IDLE=$(busctl --user get-property $SERVICE $OBJECT $INTERFACE IsIdleInhibited --json=short 2>/dev/null | jq -r '.data // "false"')
    IS_MANUAL=$(busctl --user get-property $SERVICE $OBJECT $INTERFACE ManualInhibit --json=short 2>/dev/null | jq -r '.data // "false"')
else
    IS_IDLE="off"
    IS_MANUAL="off"
fi

# Function to fetch and print the current state
get_and_print_state() {
    # Check if the service exists on the bus
    if busctl --user status "$SERVICE" &>/dev/null; then
        local idle=$(busctl --user get-property "$SERVICE" "$OBJECT" "$INTERFACE" IsIdleInhibited --json=short 2>/dev/null | jq -r '.data // "false"')
        local manual=$(busctl --user get-property "$SERVICE" "$OBJECT" "$INTERFACE" ManualInhibit --json=short 2>/dev/null | jq -r '.data // "false"')
        print_status "$idle" "$manual"
    else
        print_status "off" "off"
    fi
}

# Output Initial State
get_and_print_state

# Monitor for changes. 
# We monitor our service for signals it sends, and org.freedesktop.DBus for name owner changes.
busctl --user monitor "$SERVICE" "org.freedesktop.DBus" --json=short | while read -r line; do
    # Detect if our service starts or stops (NameOwnerChanged signal from the bus)
    if echo "$line" | jq -e ".member == \"NameOwnerChanged\" and .payload.data[0] == \"$SERVICE\"" >/dev/null; then
        get_and_print_state
    
    elif echo "$line" | jq -e ".member == \"PropertiesChanged\" and .path == \"$OBJECT\"" >/dev/null; then
        NEW_IDLE="$(echo "$line" | jq -r '.payload.data[0].IsIdleInhibited.data')"
        NEW_MANUAL="$(echo "$line" | jq -r '.payload.data[0].IsManuallyInhibited.data')"
        
        if [[ -n "$NEW_IDLE" && -n "$NEW_MANUAL" ]]; then
            print_status "$NEW_IDLE" "$NEW_MANUAL"
        else
            # Fallback to manual poll if payload parsing fails
            get_and_print_state
        fi
    fi
done
