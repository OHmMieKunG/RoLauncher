"""Type-check the real Windows adapter and its callers using cached dependencies."""
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parent.parent
subprocess.run(["cargo", "check", "--locked", "--offline", "--all-targets"], cwd=root, check=True)
print("Windows adapter, automation and regression fixtures checked against real dependencies; no live actions.")
