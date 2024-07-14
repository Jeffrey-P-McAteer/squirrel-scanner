
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
    - https://unix.stackexchange.com/a/52645
    - `sudo pacman -S v4l-utils`
    - `sudo pacman -S ffmpeg`

# Software Pipeline Setup

 - Clone this repository to `/opt/squirrel-scanner`
 - run `sudo python install-systemd-services.py`


# SW/HW Research one-liners


```bash
# List cameras
sudo v4l2-ctl --list-devices

# send webcam frames to local framebuffer
sudo ffmpeg -i /dev/video2 -pix_fmt bgra -f fbdev /dev/fb0

# Send webcam frames to entire network over multicast
# address 239.0.0.1 port 1234
sudo ffmpeg -i /dev/video2 -c:v libx264 -preset ultrafast -qp 0 -f mpegts  'udp://239.0.0.1:1234?ttl=13&localaddr=192.168.5.36'
# same thing, with filters like these: https://stackoverflow.com/a/9570992
sudo ffmpeg -i /dev/video2 -vf "transpose=1" -c:v libx264 -preset ultrafast -qp 0 -f mpegts  'udp://239.0.0.1:1234?ttl=13'
# Spawn the above as a small daemon
sudo systemd-run --uid=0 --gid=0 --nice=-1 --working-directory=/tmp -- sudo ffmpeg -i /dev/video2 -vf "transpose=1" -c:v libx264 -preset ultrafast -qp 0 -f mpegts  'udp://239.0.0.1:1234?ttl=13'

# Play back w/ VLC / MPV
mpv udp://239.0.0.1:1234
ffplay udp://239.0.0.1:1234

```



