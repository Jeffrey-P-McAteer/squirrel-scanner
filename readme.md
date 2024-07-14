
# Squirrel-Scanner


# Hardware Setup

 - 1 laptop
 - 1 USB webcam
 - 1 TB+ USB harddrive (optional, for long-term video storage)

# Operating System Setup

 - Install Arch Linux on the laptop (https://wiki.archlinux.org/title/installation_guide)
    - User: `user`
        - `useradd -m -G wheel -s /bin/bash user`
    - Hostname: `squirrel-scanner`
 - Boot into laptop
 - Install infrastructure tools
    - `sudo pacman -S iwd`
    - Set `AutoConnect=true` in `/etc/iwd/main.conf` (https://unix.stackexchange.com/a/623037)
 - Install development tools
    - `sudo pacman -S base-devel git vim rustup python python-pip sudo less tar findutils`
 - Install remote infrastructure
    - `sudo pacman -S openssh && sudo systemctl enable --now sshd`

 - Misc
    - `sudo ln -sf ../run/systemd/resolve/stub-resolv.conf /etc/resolv.conf`
    - `sudo systemctl enable --now systemd-resolved.service`




# Software Pipeline Setup

 - Clone this repository to `/opt/squirrel-scanner`
 - run `sudo python install-systemd-services.py`






