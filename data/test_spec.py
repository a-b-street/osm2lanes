#!/bin/env python3
"""
Test lane spec.
"""
import json
import typing
import unittest
import yaml
from pathlib import Path

from jsonschema import validate

with Path("data/tests.yml").open() as input_file:
    CONFIGURATIONS: list[dict[str, typing.Any]] = [
        test for test in yaml.safe_load(input_file)
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
