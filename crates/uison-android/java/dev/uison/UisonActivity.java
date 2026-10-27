package dev.uison;

import android.app.Activity;
import android.content.Context;
import android.content.Intent;
import android.database.Cursor;
import android.net.Uri;
import android.provider.OpenableColumns;
import android.content.res.Configuration;
import android.graphics.Color;
import android.os.Build;
import android.os.Bundle;
import android.system.Os;
import android.view.View;
import android.view.Window;
import android.view.WindowManager;
import java.io.File;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.io.OutputStream;

/**
 * A uison app window. All logic is in Rust (libmain.so): here only a full-screen
 * window (under the system bars) and passing events to {@link UisonView}.
 */
public final class UisonActivity extends Activity {
    private static boolean loaded;

    private UisonView view;
    private static final int PICK_FILE = 1;
    /** The app's action waiting for the chosen file. */
    private String pickAction;

    /** The system's document chooser (Files, Drive…); the result comes to onActivityResult. */
    void pickFile(String action) {
        pickAction = action;
        Intent i = new Intent(Intent.ACTION_OPEN_DOCUMENT).addCategory(Intent.CATEGORY_OPENABLE).setType("*/*");
        try {
            startActivityForResult(i, PICK_FILE);
        } catch (Exception e) {
            pickAction = null;
            view.picked(action, "");
        }
    }

    @Override
    protected void onActivityResult(int request, int result, Intent data) {
        super.onActivityResult(request, result, data);
        if (request != PICK_FILE || pickAction == null) {
            return;
        }
        String action = pickAction;
        pickAction = null;
        Uri uri = result == RESULT_OK && data != null ? data.getData() : null;
        view.picked(action, uri == null ? "" : copyPicked(uri));
    }

    /** A copy of the chosen document in files/picked, readable by this app only; "" — it couldn't be read. */
    private String copyPicked(Uri uri) {
        String name = "picked";
        try (Cursor c = getContentResolver().query(uri, new String[] {OpenableColumns.DISPLAY_NAME}, null, null, null)) {
            if (c != null && c.moveToFirst() && c.getString(0) != null) {
                name = c.getString(0).replace('/', '_').replace('\\', '_');
            }
        } catch (Exception e) {
            // no name — a neutral one
        }
        File dir = new File(getFilesDir(), "picked");
        dir.mkdirs();
        File f = new File(dir, name);
        try (InputStream in = getContentResolver().openInputStream(uri); OutputStream out = new FileOutputStream(f)) {
            byte[] buf = new byte[16384];
            for (int n; (n = in.read(buf)) > 0; ) {
                out.write(buf, 0, n);
            }
        } catch (Exception e) {
            f.delete();
            return "";
        }
        f.setReadable(false, false);
        f.setReadable(true, true);
        f.setWritable(false, false);
        f.setWritable(true, true);
        return f.getPath();
    }

    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        if (!loaded) {
            // App files (assets) go to a folder that becomes the current one: a C/C++ program
            // finds its JSON and images as on a computer (UISON_FILES → chdir in JNI_OnLoad).
            File dir = files(this);
            try {
                Os.setenv("UISON_FILES", dir.getPath(), true);
            } catch (Exception e) {
                // without a current folder files won't be found, the rest works
            }
            System.loadLibrary("main"); // JNI_OnLoad: main() registers the app (uison::run)
            loaded = true;
        }
        Window w = getWindow();
        // Draw under the status bar, navigation bar and cutout; the padding comes from insets.
        w.setStatusBarColor(Color.TRANSPARENT);
        w.setNavigationBarColor(Color.TRANSPARENT);
        w.getAttributes().layoutInDisplayCutoutMode = WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES;
        if (Build.VERSION.SDK_INT >= 30) {
            w.setDecorFitsSystemWindows(false);
        } else {
            w.getDecorView().setSystemUiVisibility(View.SYSTEM_UI_FLAG_LAYOUT_STABLE
                    | View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION | View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN);
        }
        if (Build.VERSION.SDK_INT >= 29) {
            w.setNavigationBarContrastEnforced(false);
        }
        view = new UisonView(this, getIntent().getExtras());
        setContentView(view);
        view.requestFocus();
        appearance(getResources().getConfiguration());
    }

    /** assets → filesDir/assets; copied again after a new APK version is installed. */
    private static File files(Context c) {
        File dir = new File(c.getFilesDir(), "assets");
        try {
            long stamp = c.getPackageManager().getPackageInfo(c.getPackageName(), 0).lastUpdateTime;
            File mark = new File(dir, ".stamp-" + stamp);
            if (mark.exists()) {
                return dir;
            }
            dir.mkdirs();
            for (File f : dir.listFiles()) {
                f.delete();
            }
            byte[] buf = new byte[65536];
            for (String name : c.getAssets().list("")) {
                try (InputStream in = c.getAssets().open(name); OutputStream out = new FileOutputStream(new File(dir, name))) {
                    for (int n; (n = in.read(buf)) > 0; ) {
                        out.write(buf, 0, n);
                    }
                } catch (Exception e) {
                    // a folder in assets (images/, system webkit/) — skip
                }
            }
            mark.createNewFile();
        } catch (Exception e) {
            // no space or permissions — no files
        }
        return dir;
    }

    @Override
    public void onConfigurationChanged(Configuration c) {
        super.onConfigurationChanged(c);
        appearance(c);
    }

    /** System theme → Rust; status and navigation bar icons are dark on a light background. */
    private void appearance(Configuration c) {
        boolean dark = (c.uiMode & Configuration.UI_MODE_NIGHT_MASK) == Configuration.UI_MODE_NIGHT_YES;
        View d = getWindow().getDecorView();
        int light = View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR | View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR;
        d.setSystemUiVisibility(dark ? d.getSystemUiVisibility() & ~light : d.getSystemUiVisibility() | light);
        view.appearance(dark);
    }

    @Override
    protected void onPause() {
        super.onPause();
        view.paused(true);
    }

    @Override
    protected void onResume() {
        super.onResume();
        view.paused(false);
    }

    @Override
    public void onBackPressed() {
        if (!view.back()) {
            super.onBackPressed();
        }
    }
}
