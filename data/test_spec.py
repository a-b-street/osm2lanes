#!/bin/env python3
"""
Test lane spec.
"""
import json
import typing
import unittest
from pathlib import Path

from jsonschema import validate

CONFIGURATIONS: list[dict[str, typing.Any]] = [
    test for test in json.loads(Path("data/tests.json").read_text())
]
SCHEMA = json.loads(Path("data/spec-lanes.json").read_text())


class TestSpec(unittest.TestCase):
    def test_spec(self):
        for config in CONFIGURATIONS:
            with self.subTest(
                **{
                    k: v
                    for k, v in config.items()
                    if k in ("way_id", "link", "comment")
                }
            ):
                validate(instance=config["output"], schema=SCHEMA)


if __name__ == "__main__":
    unittest.main()
