#!/usr/bin/env python

import os
import sys
import subprocess

def main(args=sys.argv):
  os.chdir(os.path.dirname(__file__))
  this_dir = os.path.dirname(__file__)

  service_file = '/etc/systemd/system/squirrel-scanner.service'
  with open(service_file, 'w') as fd:
    fd.write(f'''
[Unit]
Description=Squirrel Scanner
StartLimitIntervalSec=1

[Service]
Type=simple
Restart=always
RestartSec=1
User=user
ExecStart={this_dir}/target/release/squirrel-scanner
RuntimeMaxSec=180m
StandardError=journal
StandardOutput=journal
StandardInput=null

[Install]
WantedBy=multi-user.target
'''.strip())

  subprocess.run([
    'systemctl', 'daemon-reload',
  ], check=True)
  subprocess.run([
    'systemctl', 'enable', 'squirrel-scanner.service',
  ], check=True)

  service_file = '/etc/systemd/system/squirrel-scanner-updater.service'
  with open(service_file, 'w') as fd:
    fd.write(f'''
[Unit]
Description=Squirrel Scanner Updater
StartLimitIntervalSec=60

[Service]
Type=simple
Restart=always
RestartSec=60
User=user
ExecStart={this_dir}/do-selfupdate.py
RuntimeMaxSec=180m
StandardError=journal
StandardOutput=journal
StandardInput=null

[Install]
WantedBy=multi-user.target
'''.strip())

  subprocess.run([
    'systemctl', 'daemon-reload',
  ], check=True)
  subprocess.run([
    'systemctl', 'enable', 'squirrel-scanner-updater.service',
  ], check=True)



if __name__ == '__main__':
  main()
