#!/usr/bin/env python

import os
import sys
import subprocess

def main(args=sys.argv):
  os.chdir(os.path.dirname(__file__))

  external_disk_part = '/dev/disk/by-partuuid/2db8d4c9-b33a-4a7e-a0bb-50ab127aedbc'
  external_disk_directory = '/mnt'
  external_disk_canary_file = '/mnt/ext-1tb-hdd.txt'

  if os.path.exists(external_disk_part) and not os.path.exists(external_disk_canary_file):
    print(f'Mounting {external_disk_part} to {external_disk_directory}')
    subprocess.run([
      'sudo', 'mount', external_disk_part, external_disk_directory
    ], check=True)
    subprocess.run([
      'sudo', 'chown', '-R', f'{os.getgid()}:{os.getuid()}', external_disk_directory
    ], check=True)





if __name__ == '__main__':
  main()
