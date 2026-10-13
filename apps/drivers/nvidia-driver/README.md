# NVIDIA Driver

Proprietary NVIDIA graphics driver for GeForce and other NVIDIA GPUs, installed from the Debian repositories with Secure Boot support.

## Notes

- **Install** looks for an NVIDIA GPU (it stops with an error when there is none), asks `nvidia-detect` for the right driver package and installs it with `apt`. If `nvidia-detect` gives no answer, the plain `nvidia-driver` package is used.
- To force another package (for example a legacy `nvidia-tesla-470-driver` for an old card), start the tool with the `NVIDIA_PACKAGE` environment variable set.
- With **Secure Boot** on, the driver module must be signed. The install creates a signing key in `/var/lib/dkms`, asks you for a one-time password and registers the key with the firmware. At the next reboot a blue **MOK Management** screen appears: choose **Enroll MOK**, **Continue**, then type the same password. Without this step the driver does not load.
- The `contrib`, `non-free` and `non-free-firmware` apt components must be enabled (they are by default on dreamos). Apt sources are never edited.
- Reboot after the install to load the driver.
- **Uninstall** removes the driver packages and goes back to the open source nouveau driver. The signing key is kept.
- No hybrid graphics (Optimus) configuration is done.

More information on [the Debian wiki](https://wiki.debian.org/NvidiaGraphicsDrivers).
