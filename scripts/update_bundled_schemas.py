#!/usr/bin/env python3
"""Refresh the JSON Schemas bundled into Ferrite (assets/schemas/).

Development-time only: needs network. Ferrite itself never fetches schemas.

Each schema is downloaded, then trimmed so the binary stays small and
validation stays offline:
  * annotation-only keywords (description, examples, ...) are removed;
    property *names* are never touched;
  * `$ref`s to other URLs are dropped (that sub-schema then accepts anything),
    because the offline validator cannot resolve them;
  * the result is minified.

Usage:  python scripts/update_bundled_schemas.py
Then update the retrieval date in docs/devtools-credits.md.
"""

import json
import pathlib
import sys
import urllib.request

SOURCES = {
    "package.json": "https://www.schemastore.org/package.json",
    "tsconfig.json": "https://www.schemastore.org/tsconfig.json",
    "cargo.json": "https://www.schemastore.org/cargo.json",
    "github-workflow.json": "https://www.schemastore.org/github-workflow.json",
    "docker-compose.json": "https://raw.githubusercontent.com/compose-spec/compose-go/master/schema/compose-spec.json",
    "pyproject.json": "https://raw.githubusercontent.com/SchemaStore/schemastore/master/src/schemas/json/pyproject.json",
}

ANNOTATIONS = {
    "description", "markdownDescription", "deprecationMessage", "examples",
    "title", "default", "$comment", "enumDescriptions", "markdownEnumDescriptions",
    "defaultSnippets", "errorMessage", "patternErrorMessage", "deprecated",
}
# keyword -> how its value holds sub-schemas
NAMED = {"properties", "patternProperties", "definitions", "$defs", "dependentSchemas"}
LISTS = {"allOf", "anyOf", "oneOf", "prefixItems"}
SINGLE = {
    "items", "additionalItems", "additionalProperties", "not", "if", "then", "else",
    "contains", "propertyNames", "unevaluatedItems", "unevaluatedProperties",
}

dropped_refs = []


def is_remote(ref):
    return not ref.startswith("#")


def trim(schema, where="#"):
    if isinstance(schema, list):  # e.g. "items": [ ... ] (draft-07 tuple form)
        return [trim(s, f"{where}/{i}") for i, s in enumerate(schema)]
    if not isinstance(schema, dict):
        return schema
    out = {}
    for key, val in schema.items():
        if key in ANNOTATIONS or key.startswith("x-"):
            continue
        if key == "$ref" and isinstance(val, str) and is_remote(val):
            dropped_refs.append((where, val))
            continue
        if key in NAMED and isinstance(val, dict):
            out[key] = {k: trim(v, f"{where}/{key}/{k}") for k, v in val.items()}
        elif key == "dependencies" and isinstance(val, dict):
            out[key] = {k: (v if isinstance(v, list) else trim(v, f"{where}/{key}/{k}")) for k, v in val.items()}
        elif key in LISTS and isinstance(val, list):
            out[key] = [trim(v, f"{where}/{key}/{i}") for i, v in enumerate(val)]
        elif key in SINGLE:
            out[key] = trim(val, f"{where}/{key}")
        else:
            out[key] = val
    return out


def main():
    root = pathlib.Path(__file__).resolve().parent.parent
    dest = root / "assets" / "schemas"
    dest.mkdir(parents=True, exist_ok=True)
    total = 0
    for name, url in SOURCES.items():
        with urllib.request.urlopen(url, timeout=30) as r:
            raw = json.load(r)
        before = len(dropped_refs)
        text = json.dumps(trim(raw), separators=(",", ":"), ensure_ascii=False)
        (dest / name).write_text(text + "\n", encoding="utf-8", newline="\n")
        total += len(text.encode("utf-8")) + 1
        print(f"{name}: {len(text.encode('utf-8'))} bytes, {len(dropped_refs) - before} remote $ref dropped")
    for where, ref in dropped_refs:
        print(f"  dropped {ref} at {where}")
    print(f"total {total} bytes")
    return 0 if total < 500 * 1024 else 1


if __name__ == "__main__":
    sys.exit(main())
