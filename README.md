# Pwm
A cross platform password manager written in pure rust

Uses AES encryption to keep passwords encrypted in memory at all times until it is specifically retrieved, the password manager does not have the information to decrypt until you specifically enter your password

All passwords, hashes and other critical data are zeroed out in memory after they are done being used

Passwords are sent to the system clipboard and are never visually visible, clearing out the system keyboard can be done with the clear password button, however if your system stores clipboard history that is your responsibility to clear

# Warning

Non-minor version bumps are likely to break serialization, remaking a vault is required on a version bump (use csv export/import)
