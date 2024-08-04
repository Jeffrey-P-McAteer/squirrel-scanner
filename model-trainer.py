
import os
import sys
import subprocess
import shutil

site_packages = os.path.join(os.path.dirname(__file__), 'model-site-packages')
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


