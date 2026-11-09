#!/usr/bin/env python3
"""Sanitized adversarial model-output regressions; no provider access."""
import copy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / 'docs/research/intelligent-search-evidence'


class SemanticContract(unittest.TestCase):
    def setUp(self):
        self.value = json.loads((EVIDENCE / 'luna-assessment.json').read_text())

    def check_result(self, value, valid):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'assessment.json'
            path.write_text(json.dumps(value), encoding='utf-8')
            result = subprocess.run([sys.executable, str(ROOT / 'scripts/check_semantic_assessment.py'),
                                     str(path), '--candidates', str(EVIDENCE / 'luna-candidates.json')],
                                    capture_output=True, text=True, check=False)
            self.assertEqual(result.returncode == 0, valid)
            if valid:
                self.assertFalse(json.loads(result.stdout)['destructive_authority'])

    def test_actual_synthetic_luna_result_validates(self):
        self.check_result(self.value, True)

    def test_foreign_identity_or_evidence_cannot_enter_result(self):
        for field, value in [('native_session_id', '00000000-0000-4000-8000-00000000000c'),
                             ('target', {'provider': 'codex', 'class': 'session', 'token': 'forged'}),
                             ('supporting_evidence_refs', ['foreign-evidence'])]:
            changed = copy.deepcopy(self.value)
            changed['assessments'][0][field] = value
            self.check_result(changed, False)

    def test_prompt_injection_authority_fields_are_rejected(self):
        changed = copy.deepcopy(self.value)
        changed['execute_deletion'] = True
        self.check_result(changed, False)
        changed = copy.deepcopy(self.value)
        changed['assessments'][0]['confidence'] = '100% certain'
        self.check_result(changed, False)

    def test_budgets_types_and_duplicate_candidates_refuse(self):
        for field, value in [('schema_version', True), ('requests_used', 9), ('context_bytes', 65537)]:
            changed = copy.deepcopy(self.value)
            changed[field] = value
            self.check_result(changed, False)
        changed = copy.deepcopy(self.value)
        changed['assessments'].append(changed['assessments'][0])
        self.check_result(changed, False)


if __name__ == '__main__':
    unittest.main()
