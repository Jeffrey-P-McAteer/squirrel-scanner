
import os
import sys
import subprocess
import shutil

site_packages = os.path.join(os.path.dirname(__file__), 'model-site-packages')

if os.path.exists('/mnt/scratch/tmp'): # Jeff's laptop, site-packages ends up being 4.5gb!!! so it goes on the scratch disk.
  site_packages = '/mnt/scratch/squirrel-scanner/model-site-packages'

os.makedirs(site_packages, exist_ok=True)
sys.path.append(site_packages)

try:
  import ultralytics
except:
  subprocess.run([
    sys.executable, '-m', 'pip', 'install', f'--target={site_packages}', 'ultralytics'
  ])
  import ultralytics


print(f'ultralytics = {ultralytics}')
ultralytics.checks()

print(f'ultralytics.YOLO = {ultralytics.YOLO}')



try:
  import anylabeling
except:
  subprocess.run([
    sys.executable, '-m', 'pip', 'install', f'--target={site_packages}', 'anylabeling', 'anylabeling-gpu'
  ])
  import anylabeling

print(f'anylabeling = {anylabeling}')

subp_env = dict(os.environ)
subp_env['PYTHONPATH'] = subp_env.get('PYTHONPATH', '')+':'+site_packages+':'
subprocess.run([
  sys.executable, '-m', 'anylabeling'
], env=subp_env)


