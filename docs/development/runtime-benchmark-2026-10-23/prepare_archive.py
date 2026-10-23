import hashlib
import json
import zipfile
from pathlib import Path

source = Path('/tmp/nhsbsa_dmd_9.3.0_20260928000001')
destination = Path('/tmp/medtracker-benchmark/release.zip')
files = sorted(source.glob('*.xml')) + sorted(source.glob('*GTIN.zip'))
manifest = []
with zipfile.ZipFile(destination, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=6) as archive:
    for path in files:
        archive.write(path, path.name)
        manifest.append({'name': path.name, 'bytes': path.stat().st_size,
                         'sha256': hashlib.sha256(path.read_bytes()).hexdigest()})
digest = hashlib.sha256(destination.read_bytes()).hexdigest()
result = {'source': str(source), 'archive': str(destination), 'archive_bytes': destination.stat().st_size,
          'expanded_bytes': sum(row['bytes'] for row in manifest), 'sha256': digest, 'files': manifest}
Path('/tmp/medtracker-benchmark/archive.json').write_text(json.dumps(result, indent=2))
print(json.dumps({key: value for key, value in result.items() if key != 'files'}, indent=2))
