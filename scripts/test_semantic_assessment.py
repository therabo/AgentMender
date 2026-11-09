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

    def check_result(self, value, valid, candidates=None):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'assessment.json'
            path.write_text(json.dumps(value), encoding='utf-8')
            candidate_path = EVIDENCE / 'luna-candidates.json'
            if candidates is not None:
                candidate_path = Path(directory) / 'candidates.json'
                candidate_path.write_text(json.dumps(candidates), encoding='utf-8')
            result = subprocess.run([sys.executable, str(ROOT / 'scripts/check_semantic_assessment.py'),
                                     str(path), '--candidates', str(candidate_path)],
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

    def test_partial_unknown_and_unavailable_cannot_support_session_rejection(self):
        value = copy.deepcopy(self.value)
        value['assessments'][0].update(assessment='not_relevant', confidence='strong', evidence_basis='complete_absence')
        candidates = json.loads((EVIDENCE / 'luna-candidates.json').read_text())
        for coverage in ('partial', 'unsupported', 'unknown', 'unavailable_or_partial'):
            candidates[0].update(content_coverage=coverage, hard_filters_verified=True, source_revalidated=True,
                                 negative_evidence_refs=candidates[0]['evidence_refs'])
            self.check_result(value, False, candidates)
        candidates[0].update(content_coverage='complete', negative_evidence_refs=[])
        self.check_result(value, False, candidates)
        candidates[0]['negative_evidence_refs'] = candidates[0]['evidence_refs']
        self.check_result(value, True, candidates)
        for field in ('source_revalidated', 'hard_filters_verified'):
            changed = copy.deepcopy(candidates)
            changed[0][field] = False
            self.check_result(value, False, changed)

    def test_contradiction_does_not_supply_missing_coverage_or_negative_proof(self):
        value = copy.deepcopy(self.value)
        value['assessments'][0].update(assessment='not_relevant', evidence_basis='explicit_contradiction')
        self.check_result(value, False)
        value['assessments'][0].update(assessment='relevant')
        self.check_result(value, True)
        value['assessments'][0].update(assessment='insufficient', evidence_basis='insufficient', missing_evidence=[])
        self.check_result(value, False)


if __name__ == '__main__':
    unittest.main()
