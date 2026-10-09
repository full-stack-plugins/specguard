"""Validate local domain golden fixtures; no authentication claim."""
import copy
import json
from pathlib import Path
from jsonschema import Draft202012Validator
root = Path(__file__).resolve().parents[1]
for name, fixture in [('baseline', 'baseline'), ('obligations', 'obligations')]:
    schema = json.loads((root / f'schemas/specguard-domain/{name}.json').read_text())
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema)
    value = json.loads((root / f'fixtures/handoffs/{fixture}.json').read_text())
    validator.validate(value)
    for field in schema['required']:
        missing = copy.deepcopy(value)
        missing.pop(field)
        assert list(validator.iter_errors(missing)), field
    unknown = copy.deepcopy(value)
    unknown['testPassed'] = True
    assert list(validator.iter_errors(unknown))
    unknown['apiVersion'] = 'future/v2'
    assert list(validator.iter_errors(unknown))
    print(name, 'valid, required fields, unknown fields and version checks passed')
