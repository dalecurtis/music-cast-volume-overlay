We're going to build this tool in several steps to make sure it works as intended.


Phase 1:
* Initialize a basic Rust project that produces an application that runs in the background with a tray icon that allows a user to right click the tray icon and exit the application.
* Generate something silly for the tray icon for now.

Phase 2:
* Identify the MusicCast receiver on the network via SSDP (port 1900). Simply print its address and then dump the status of the reciever via http://[RECEIVER_IP]/YamahaExtendedControl/v1/main/getStatus to the console.

Phase 3:
* Register as an extended controller via `YamahaExtendedControl` interface using : X-AppName: [YourAppName] and X-AppPort: [YourUDPPort]. http://{IP}/YamahaExtendedControl/v1/main/prepareEvent?device=udp&port={YOUR_PORT
* Listen for broadcast events and dump the broadcast event to the console.
* Every 10 minutes re-register the endpoint to prevent timeout.
* If a suspend/resume occurs, wait a few seconds, then rerun receiver detection and registration.
* TODO: Does the reciever broadcast a power off event that we can listen for as well?

Phase 4:
* Display a black rectangle in the lower right corner of the monitor showing the current volume (`actual_volume`) using the Consolas font with a 48pt size and the unicode volume character 🔊. Hide the display after 2 seconds of inactivity, or update it if the volume changes again during the 2 second window.

Phase 5:
* Add command line options to run without a console window.
