#!/usr/bin/env python3
"""Validate the bounded semantic JSON contract and exact candidate allowlist.
This validates data, not source freshness, hard filters or deletion authority.
The caller must reissue the original deterministic query before presentation.
"""
import argparse
import json
import re
from pathlib import Path


def validate(schema, value, root):
    supported = {'$schema', '$ref', '$defs', 'title', 'type', 'properties', 'required',
                 'additionalProperties', 'enum', 'const', 'minimum', 'maximum',
                 'minLength', 'maxLength', 'pattern', 'maxItems', 'uniqueItems', 'items'}
    if set(schema) - supported:
        raise ValueError('unsupported contract keyword')
    if '$ref' in schema:
        reference = schema['$ref']
        if not reference.startswith('#/$defs/'):
            raise ValueError('nonlocal schema reference')
        return validate(root['$defs'][reference.split('/')[-1]], value, root)
    kind = schema.get('type')
    types = {'object': lambda x: isinstance(x, dict), 'array': lambda x: isinstance(x, list),
             'string': lambda x: isinstance(x, str), 'integer': lambda x: type(x) is int,
             'boolean': lambda x: type(x) is bool}
    if kind is not None and (kind not in types or not types[kind](value)):
        raise ValueError('invalid contract type')
    if 'const' in schema and (type(value) is not type(schema['const']) or value != schema['const']):
        raise ValueError('invalid constant')
    if 'enum' in schema and value not in schema['enum']:
        raise ValueError('invalid category')
    if isinstance(value, dict):
        properties = schema.get('properties', {})
        if not set(schema.get('required', [])).issubset(value):
            raise ValueError('missing required fields')
        if schema.get('additionalProperties') is False and set(value) - set(properties):
            raise ValueError('unknown fields')
        for key, child in value.items():
            if key in properties:
                validate(properties[key], child, root)
    if isinstance(value, str):
        if not schema.get('minLength', 0) <= len(value) <= schema.get('maxLength', len(value)):
            raise ValueError('string limit')
        if 'pattern' in schema and not re.fullmatch(schema['pattern'], value):
            raise ValueError('identity pattern')
    if type(value) is int and not schema.get('minimum', value) <= value <= schema.get('maximum', value):
        raise ValueError('numeric budget')
    if isinstance(value, list):
        if len(value) > schema.get('maxItems', len(value)):
            raise ValueError('array budget')
        encoded = [json.dumps(item, sort_keys=True) for item in value]
        if schema.get('uniqueItems') and len(encoded) != len(set(encoded)):
            raise ValueError('duplicate references')
        for item in value:
            validate(schema.get('items', {}), item, root)


def bounded_json(path):
    if path.stat().st_size > 128 * 1024:
        raise ValueError('input byte limit')
    def unique(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError('duplicate JSON field')
            result[key] = value
        return result
    return json.loads(path.read_text(encoding='utf-8'), object_pairs_hook=unique)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('assessment', type=Path)
    parser.add_argument('--candidates', type=Path, required=True,
                        help='trusted [{target,native_session_id,evidence_refs}] handoff')
    args = parser.parse_args()
    contract = bounded_json(Path(__file__).resolve().parents[1] / 'docs/session-relevance-output.schema.json')
    result = bounded_json(args.assessment)
    candidates = bounded_json(args.candidates)
    validate(contract, result, contract)
    seen = set()
    for assessment in result['assessments']:
        identity = json.dumps([assessment['target'], assessment['native_session_id']], sort_keys=True)
        if identity in seen:
            raise ValueError('duplicate candidate assessment')
        seen.add(identity)
        allowed = next((c for c in candidates if c['target'] == assessment['target'] and c['native_session_id'] == assessment['native_session_id']), None)
        if allowed is None or not set(assessment['supporting_evidence_refs']).issubset(allowed['evidence_refs']):
            raise ValueError('candidate or evidence not in trusted handoff')
    print(json.dumps({'schema_valid': True, 'candidate_allowlist_valid': True,
                      'assessments': len(result['assessments']), 'source_revalidation_required': True,
                      'destructive_authority': False}))


if __name__ == '__main__':
    main()
