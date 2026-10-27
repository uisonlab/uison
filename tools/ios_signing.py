"""Signing for the phone: prints "<certificate sha1> <profile path> <team>".

Looks for a development profile with a wildcard App ID ("<team>.*") that includes
a certificate from the keychain. Arguments (optional): certificate sha1, profile path.
"""

import glob
import hashlib
import os
import plistlib
import subprocess
import sys


def identities():
    out = subprocess.run(["security", "find-identity", "-v", "-p", "codesigning"], capture_output=True, text=True).stdout
    return {line.split()[1] for line in out.splitlines() if '"' in line}


def profile(path):
    der = subprocess.run(["security", "cms", "-D", "-i", path], capture_output=True).stdout
    return plistlib.loads(der)


def main():
    want_id = sys.argv[1] if len(sys.argv) > 1 else ""
    want_profile = sys.argv[2] if len(sys.argv) > 2 else ""
    ids = identities()
    dirs = ["~/Library/Developer/Xcode/UserData/Provisioning Profiles", "~/Library/MobileDevice/Provisioning Profiles"]
    paths = [want_profile] if want_profile else [p for d in dirs for p in glob.glob(os.path.expanduser(d) + "/*.mobileprovision")]
    for path in paths:
        p = profile(path)
        app_id = p["Entitlements"]["application-identifier"]
        team = p["TeamIdentifier"][0]
        if not want_profile and app_id != team + ".*":
            continue
        for cert in p["DeveloperCertificates"]:
            sha = hashlib.sha1(cert).hexdigest().upper()
            if sha in ids and (not want_id or want_id.upper() == sha):
                print(sha, path.replace(" ", "\\ "), team)
                return
    sys.exit("no development profile with a wildcard App ID and a matching certificate (set UISON_IOS_IDENTITY / UISON_IOS_PROFILE)")


main()
