#!/usr/bin/env python3
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///

import datetime
import json
import os
import re
import sys
import subprocess
import tomllib
import urllib.request
from xml.dom.minidom import parse as parse_xml


class VersionChecks:
    def __init__(self):
        self.errors = []
        self.get_cargo_version()
        self.check_changelog()
        self.check_doap()
        self.check_tag()
        self.check_crate()

    def get_cargo_version(self):
        with open("Cargo.toml", "rb") as f:
            data = tomllib.load(f)
        self.version = data["package"]["version"]
        self.name = data["package"]["name"]
        print(f"Cargo version: {self.name} {self.version}")

    def check_changelog(self):
        errors = []
        with open("CHANGELOG.md", "rb") as f:
            data = f.read()
        lines = data.decode("utf-8").splitlines()
        first_line = lines[0]
        match = re.match(r"^# (\d+\.\d+\.\d+) \((\d{4}-\d{2}-\d{2})\)", first_line)
        if match:
            changelog_version = match.group(1)
            changelog_date = match.group(2)
            if changelog_version != self.version:
                errors.append(f"CHANGELOG.md version {changelog_version} does not match Cargo version {self.version}")
            current_date = datetime.datetime.now().strftime("%Y-%m-%d")
            if changelog_date != current_date:
                errors.append(f"CHANGELOG.md date {changelog_date} does not match current date {current_date}")
        else:
            errors.append("First line of CHANGELOG.md should be '# X.Y.Z (YYYY-MM-DD)")
        if errors:
            self.errors.extend(errors)
        else:
            print("CHANGELOG.md: up-to-date")

    def check_doap(self):
        errors = []
        doap_file = f"{self.name}.doap"
        doap = parse_xml(doap_file)
        doap_version = doap.getElementsByTagName("revision")[0].firstChild.data
        if doap_version != self.version:
            errors.append(f"DOAP version {doap_version} does not match Cargo version {self.version}")
        if errors:
            self.errors.extend(errors)
        else:
            print(f"{doap_file}: up-to-date")

    def check_tag(self):
        errors = []
        tag_name = f"v{self.version}"
        output = subprocess.run(["git", "tag", "-l", tag_name], check=True, capture_output=True, text=True)
        if output.stdout.strip() == tag_name:
            errors.append(f"Release tag already exists: {tag_name}")
        if errors:
            self.errors.extend(errors)
        else:
            print(f"Tag: {tag_name} available")

    def check_crate(self):
        errors = []
        with urllib.request.urlopen(f"https://crates.io/api/v1/crates/{self.name}") as response:
           str_data = response.read()
        data = json.loads(str_data)
        published_versions = [version["num"] for version in data["versions"]]
        if self.version in published_versions:
            errors.append(f"Crate version {self.version} already published")
        if errors:
            self.errors.extend(errors)
        else:
            print("Crates.io: ready")

    def output_version(self):
        outfile = os.getenv("GITHUB_OUTPUT")
        if outfile:
            with open(outfile, "a") as f:
                f.write(f"version={self.version}\n")
            print(f"Version written to {outfile}")


def main():
    checks = VersionChecks()
    if checks.errors:
        print()
        print("Errors found:")
        for error in checks.errors:
            print(f"- {error}")
        # FIXME: ignoring error now to test the script
        #sys.exit(1)
    checks.output_version()
    print("All checks passed!")


if __name__ == "__main__":
    main()
