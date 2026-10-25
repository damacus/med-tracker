import json
import subprocess
from pathlib import Path

root = Path('/tmp/medtracker-benchmark')
results = {}
cases = [('rails','async','a'),('rails','inline','b'),('loco','async','a'),('loco','inline','b'),('rails','fork','c')]
for app,mode,trial in cases:
    if not (root/f'{app}-{mode}'/'summary.json').exists():
        continue
    results[f'{app}-{mode}'] = {}
    for table,columns in [
        ('nhs_dmd_barcodes','gtin,code,display,system,amp_code,concept_class,vmp_name'),
        ('nhs_dmd_ampp_relationships','ampp_code,amp_code')]:
        query = f"SELECT count(*),md5(string_agg(row_to_json(r)::text,chr(10) ORDER BY row_to_json(r)::text COLLATE \"C\")) FROM (SELECT {columns} FROM {table}) r"
        output = subprocess.check_output(['rtk','proxy','docker','exec',f'mt-idle-6577-{app}-db-1','psql','-U','medtracker','-d',f'import_trial_{trial}','-Atc',query],text=True)
        count,digest = output.strip().split('|')
        results[f'{app}-{mode}'][table] = {'rows':int(count),'md5':digest}
(root/'catalogue-verification.json').write_text(json.dumps(results,indent=2))
print(json.dumps(results,indent=2))
assert all(value == next(iter(results.values())) for value in results.values()), 'Imported catalogue contents differ'
