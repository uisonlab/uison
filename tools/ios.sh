#!/bin/zsh
# Build an example as an iOS app (.app) without an Xcode project.
#   tools/ios.sh <package | path to a built program> [sim|device] [--debug]
# sim (default) — aarch64-apple-ios-sim, ad-hoc signing.
# device — aarch64-apple-ios, signed with an Apple Development certificate and a profile
#   with a wildcard App ID (found automatically; to set explicitly: UISON_IOS_IDENTITY, UISON_IOS_PROFILE).
# A built program (C/C++ with libuison.a, see docs/c) is not built, only packaged.
# UISON_IOS_FILES="a.json b.json" — put files into the .app (scenarios for UISON_SCRIPT, screens, images).
# Result: target/ios/<sim|device>/<package>.app; the path is printed on the last line.
set -e
# Resolve paths from the launch folder (a built program of a crate outside the repository) before changing to the root.
[[ -f ${1:-} ]] && set -- ${1:A} "${@:2}"
cd "${0:A:h}/.."

pkg=${1:?usage: tools/ios.sh <package | binary> [sim|device] [--debug]}
bin=
[[ -f $pkg ]] && { bin=$pkg; pkg=${pkg:t}; }
kind=${2:-sim}
profile=release
[[ " $* " == *" --debug "* ]] && profile=debug
case $kind in
  sim) target=aarch64-apple-ios-sim; platform=iPhoneSimulator ;;
  device) target=aarch64-apple-ios; platform=iPhoneOS ;;
  *) echo "kind: sim | device" >&2; exit 1 ;;
esac

export IPHONEOS_DEPLOYMENT_TARGET=17.0
flags=()
[[ $profile == release ]] && flags=(--release)
[[ -z $bin ]] && { cargo build -q -p $pkg --target $target $flags >&2; bin=target/$target/$profile/$pkg; }

bundle_id=dev.uison.${pkg//_/-}
app=target/ios/$kind/$pkg.app
rm -rf $app
mkdir -p $app
cp $bin $app/$pkg
for f in ${=UISON_IOS_FILES}; do cp $f $app/; done

cat > $app/Info.plist <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key><string>$bundle_id</string>
  <key>CFBundleExecutable</key><string>$pkg</string>
  <key>CFBundleName</key><string>$pkg</string>
  <key>CFBundleDisplayName</key><string>${UISON_IOS_NAME:-$pkg}</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>CFBundleShortVersionString</key><string>0.1</string>
  <key>CFBundleSupportedPlatforms</key><array><string>$platform</string></array>
  <key>MinimumOSVersion</key><string>$IPHONEOS_DEPLOYMENT_TARGET</string>
  <key>LSRequiresIPhoneOS</key><true/>
  <key>UIDeviceFamily</key><array><integer>1</integer><integer>2</integer></array>
  <key>UIRequiredDeviceCapabilities</key><array><string>arm64</string></array>
  <key>UILaunchScreen</key><dict/>
  <key>UISupportedInterfaceOrientations</key>
  <array>
    <string>UIInterfaceOrientationPortrait</string>
    <string>UIInterfaceOrientationLandscapeLeft</string>
    <string>UIInterfaceOrientationLandscapeRight</string>
  </array>
  <key>UIApplicationSceneManifest</key>
  <dict>
    <key>UIApplicationSupportsMultipleScenes</key><false/>
    <key>UISceneConfigurations</key>
    <dict>
      <key>UIWindowSceneSessionRoleApplication</key>
      <array>
        <dict>
          <key>UISceneConfigurationName</key><string>Default</string>
          <key>UISceneDelegateClassName</key><string>UisonSceneDelegate</string>
        </dict>
      </array>
    </dict>
  </dict>
</dict>
</plist>
EOF
plutil -convert binary1 $app/Info.plist

if [[ $kind == sim ]]; then
  codesign -f -s - $app 2>/dev/null
else
  # Profile and certificate: a profile with App ID "<team>.*" whose certificate is in the keychain.
  read identity profile_path team <<<"$(python3 -I tools/ios_signing.py "${UISON_IOS_IDENTITY:-}" "${UISON_IOS_PROFILE:-}")"
  cp "$profile_path" $app/embedded.mobileprovision
  ent=target/ios/$kind/$pkg.entitlements
  cat > $ent <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>application-identifier</key><string>$team.$bundle_id</string>
  <key>com.apple.developer.team-identifier</key><string>$team</string>
  <key>get-task-allow</key><true/>
</dict>
</plist>
EOF
  codesign -f -s $identity --entitlements $ent $app 2>/dev/null
fi

echo "$pkg: $(stat -f %z $app/$pkg | awk '{printf "%.0f KB", $1/1024}') ($kind, $profile)" >&2
echo $app
