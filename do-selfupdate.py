#!/usr/bin/env python

import os
import sys
import subprocess

def main(args=sys.argv):
  os.chdir(os.path.dirname(__file__))
  before_pull_hash = subprocess.check_output(['git', 'rev-parse', '--short', 'HEAD']).decode('utf-8')
  before_pull_hash = before_pull_hash.lower().strip()

  subprocess.run([
    'git', 'pull', 'origin', 'master',
  ], check=True)

  after_pull_hash = subprocess.check_output(['git', 'rev-parse', '--short', 'HEAD']).decode('utf-8')
  after_pull_hash = after_pull_hash.lower().strip()

  if before_pull_hash != after_pull_hash:
    # We got updates, re-build and restart service!
    subprocess.run([
      'cargo', '+nightly', 'build', '--release',
    ], check=True)

    subprocess.run([
      'sudo', '-i', 'systemctl', 'restart', 'squirrel-scanner.service',
    ], check=True)





if __name__ == '__main__':
  main()
