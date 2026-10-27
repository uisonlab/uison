#!/bin/zsh
# Build an example as an Android app (APK) without Gradle.
#   tools/android.sh <package | built program> [--debug]
# The example binary is built as libmain.so (-C link-arg=-shared; the entry point is JNI_OnLoad → main),
# the Java shim crates/uison-android/java → classes.dex, the manifest → aapt2, signed with the debug key.
# UISON_ANDROID_ABIS — space-separated ABIs ("arm64-v8a armeabi-v7a" by default).
# A built program (C/C++ with libuison.a, see docs/c): a libmain.so file (one ABI — the first of UISON_ANDROID_ABIS)
# or a folder with <ABI>/libmain.so — not built, only packaged.
# UISON_ANDROID_DEBUGGABLE=1 — the app is debuggable: `adb shell run-as` can put files into its own folder
#   (an SSH key for uison-ssh-player) without building them into the APK.
# UISON_ANDROID_ASSETS="a.json b.json" — put files into assets (scenarios for UISON_SCRIPT; program files:
#   UisonActivity copies assets into a folder that becomes the current one).
# Result: target/android/<package>.apk; the path is printed on the last line.
set -e
# Resolve paths from the launch folder (a built program of a crate outside the repository) before changing to the root.
[[ -e ${1:-} ]] && set -- ${1:A} "${@:2}"
cd "${0:A:h}/.."

pkg=${1:?usage: tools/android.sh <package | libmain.so | dir> [--debug]}
prebuilt=
if [[ -e $pkg ]]; then
  prebuilt=${pkg:A}
  pkg=${${pkg:A}:h:t}  # the name comes from the program's folder
  [[ -d $prebuilt ]] && pkg=${prebuilt:t}
fi
profile=release
[[ " $* " == *" --debug "* ]] && profile=debug

SDK=${ANDROID_HOME:-$HOME/Library/Android/sdk}
NDK=${ANDROID_NDK_HOME:-$(ls -d $SDK/ndk/* | sort -V | tail -1)}
BT=$(ls -d $SDK/build-tools/* | sort -V | tail -1)
JAR=$(ls -d $SDK/platforms/android-*/android.jar | sort -V | tail -1)
CLANG=$NDK/toolchains/llvm/prebuilt/darwin-x86_64/bin
MIN_SDK=29

out=target/android/$pkg
rm -rf $out
mkdir -p $out/apk/lib $out/classes

# ---------------------------------------------------------------- Rust → libmain.so
flags=()
[[ $profile == release ]] && flags=(--release)
for abi in ${=UISON_ANDROID_ABIS:-arm64-v8a armeabi-v7a}; do
  case $abi in
    arm64-v8a) target=aarch64-linux-android; cc=aarch64-linux-android$MIN_SDK-clang ;;
    armeabi-v7a) target=armv7-linux-androideabi; cc=armv7a-linux-androideabi$MIN_SDK-clang ;;
    *) echo "unknown ABI $abi" >&2; exit 1 ;;
  esac
  mkdir -p $out/apk/lib/$abi
  if [[ -f $prebuilt ]]; then
    cp $prebuilt $out/apk/lib/$abi/libmain.so
    break  # one file means one ABI
  elif [[ -d $prebuilt ]]; then
    [[ -f $prebuilt/$abi/libmain.so ]] || { rmdir $out/apk/lib/$abi; continue; }
    cp $prebuilt/$abi/libmain.so $out/apk/lib/$abi/
  else
    var=CARGO_TARGET_${${target//-/_}:u}
    # CC/AR: crates with C inside (ring in uison-ssh-player) build it with the NDK's clang.
    env ${var}_LINKER=$CLANG/$cc ${var}_RUSTFLAGS="-C link-arg=-shared" \
      "CC_${target//-/_}=$CLANG/$cc" "AR_${target//-/_}=$CLANG/llvm-ar" \
      cargo build -q -p $pkg --target $target $flags >&2
    cp target/$target/$profile/$pkg $out/apk/lib/$abi/libmain.so
  fi
  [[ $profile == release ]] && $CLANG/llvm-strip $out/apk/lib/$abi/libmain.so
done

# ---------------------------------------------------------------- Java → classes.dex
javac -nowarn -XDsuppressNotes --release 11 -Xlint:-options -classpath $JAR -d $out/classes crates/uison-android/java/dev/uison/*.java >&2
$BT/d8 --release --min-api $MIN_SDK --lib $JAR --output $out/apk $out/classes/dev/uison/*.class >&2

# ---------------------------------------------------------------- manifest, resources, assets
cat > $out/AndroidManifest.xml <<EOF
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="dev.uison.${pkg//-/_}" android:versionCode="1" android:versionName="0.1">
  <!-- Network (the browser example, uison-ssh-player): a normal permission, granted without asking. -->
  <uses-permission android:name="android.permission.INTERNET"/>
  <application android:label="${UISON_ANDROID_NAME:-$pkg}" android:hasCode="true" android:allowBackup="false"
      android:debuggable="${${UISON_ANDROID_DEBUGGABLE:+true}:-false}"
      android:extractNativeLibs="true" android:theme="@android:style/Theme.DeviceDefault.Light.NoActionBar">
    <activity android:name="dev.uison.UisonActivity" android:exported="true" android:launchMode="singleTask"
        android:windowSoftInputMode="adjustResize"
        android:configChanges="orientation|screenSize|screenLayout|smallestScreenSize|keyboard|keyboardHidden|navigation|uiMode|density|fontScale|locale|layoutDirection">
      <intent-filter>
        <action android:name="android.intent.action.MAIN"/>
        <category android:name="android.intent.category.LAUNCHER"/>
      </intent-filter>
    </activity>
  </application>
</manifest>
EOF
assets=()
if [[ -n $UISON_ANDROID_ASSETS ]]; then
  mkdir -p $out/assets
  for f in ${=UISON_ANDROID_ASSETS}; do cp $f $out/assets/; done
  assets=(-A $out/assets)
fi
$BT/aapt2 link -o $out/base.apk --manifest $out/AndroidManifest.xml -I $JAR \
  --min-sdk-version $MIN_SDK --target-sdk-version 36 $assets >&2

# ---------------------------------------------------------------- APK: dex + .so, alignment, signing
(cd $out/apk && zip -q -r ../base.apk classes.dex lib)
$BT/zipalign -f -P 16 4 $out/base.apk $out/aligned.apk
apk=target/android/$pkg.apk
$BT/apksigner sign --ks $HOME/.android/debug.keystore --ks-pass pass:android --key-pass pass:android \
  --out $apk $out/aligned.apk
rm -f $apk.idsig

sizes=$(for so in $out/apk/lib/*/libmain.so; do printf "%s %d KB, " ${${so:h}:t} $(( $(stat -f %z $so) / 1024 )); done)
echo "$pkg: ${sizes}APK $(( $(stat -f %z $apk) / 1024 )) KB ($profile)" >&2
echo $apk
