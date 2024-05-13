# Pwm
A cross platform password manager written in pure rust

Uses AES encryption to keep passwords encrypted in memory at all times until it is specifically retrieved, the password manager does not have the information to decrypt unless you specifically enter your password

All passwords, hashes and other critical data are zeroed out in memory after they are done being used

Passwords are sent to the system clipboard and are never visually visible, clearing out the system keyboard can be done with the clear password button, however if your system stores clipboard history that is your responsibility to clear

# Warning

Version bumps are likely to break serialization, remaking a vault is required on a version bump (use csv export/import)

# Bugs
?Can't close if file manager is open

# Not active
Delete keep_hash, currently depreciated

# Todo
Clear password from clipboard after 10 seconds **Not possible on wayland**

Custom title bar:
https://github.com/emilk/egui/blob/master/examples/custom_window_frame/src/main.rs

Compress saved file?

testing for CLI