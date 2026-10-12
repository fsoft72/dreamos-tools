# Firefox

Open source web browser from Mozilla.

![Firefox](screenshot.png)

## Notes

- Installed as a real deb from the official Mozilla apt repository, which is added with a higher pin priority. Ubuntu's own `firefox` package is only a stub for the snap version.
- If the Firefox snap is installed it is **removed** after the deb is in place. Snap saves a snapshot of its data first (`snap saved` lists it). The snap profile (`~/snap/firefox`) is separate from the deb one (`~/.mozilla/firefox`), so bookmarks and passwords do not move by themselves: use Firefox Sync or copy the profile.
- Updates arrive with the normal system updates.
- Uninstall removes the package and leaves the repository in place.
- Re-run **Install** at any time to update immediately.

More information on [the project site](https://www.mozilla.org/firefox/).
