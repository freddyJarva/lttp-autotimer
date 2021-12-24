from pathlib import Path
from shutil import copyfile

DATA_DIR = Path("./postprocess/data")
DATA_DIR.mkdir(exist_ok=True)

# This lets us access the files from the python package
for jsonp in Path.cwd().glob("../src/*.json"):
    copyfile(str(jsonp), str(DATA_DIR / jsonp.name))
