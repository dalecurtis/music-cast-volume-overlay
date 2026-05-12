# music-cast-volume-overlay
Windows app to display an overlay for MusicCast volume changes

As of 2026, Yamaha receivers won't display the OSD for the current volume if the volume changes when the display is operating in HDMI 2.1 (4K120Hz, etc) mode.

This tool provides a workaround using a native Windows application which registers as a MusicCast listener and displays an equivalent UI when volume updates are broadcast via the MusicCast API.

The plan to do this is as follows:
* Use SSDP (port=1900) to to identify a MusicCast endpoint on the local network.
* Register as an extended controller via the `YamahaExtendedControl` API (refresh required every 10 minutes).
* Upon receipt of a volume change, displays black square with the contents of the `actual_volume` property that ideally looks just like the receiver's normal UI.
* If a suspend/resume event is detected, rerun discovery and registration.
