package dev.uison;

import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Context;
import android.content.Intent;
import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.graphics.Canvas;
import android.graphics.Insets;
import android.graphics.Paint;
import android.graphics.Path;
import android.graphics.Rect;
import android.graphics.RectF;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.text.InputType;
import android.view.Choreographer;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.View;
import android.view.WindowInsets;
import android.view.inputmethod.BaseInputConnection;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputConnection;
import android.view.inputmethod.InputMethodManager;

import java.io.ByteArrayOutputStream;
import java.io.InputStream;

/**
 * The uison surface: drawing ({@link #onDraw}) and input are passed to Rust;
 * Rust calls back {@link #keyboard}, {@link #frame}, {@link #tick}, {@link #damage}.
 * Native methods are registered by JNI_OnLoad (RegisterNatives).
 */
final class UisonView extends View implements Choreographer.FrameCallback {
    private static native void nativeCreate(UisonView view, float density, String env, String script);
    private static native void nativeResize(int w, int h);
    private static native void nativeInsets(int top, int right, int bottom, int left, int ime);
    private static native void nativeDraw(Canvas c);
    private static native void nativePointer(int action, float x, float y, int tool, int buttons);
    private static native void nativeScroll(float x, float y, float dy);
    private static native boolean nativeKey(int code, int unicode, int meta);
    private static native void nativeText(String text);
    private static native void nativeFrame(long nanos);
    private static native void nativeTick();
    private static native void nativeAppearance(boolean dark);
    private static native boolean nativeBack();
    private static native void nativeStep();
    private static native void nativeMessage(String json);

    /** On-screen keyboard type (EditorInfo.inputType) or -1 — not needed. */
    private int input = -1;
    private boolean framePending;
    private final Runnable tickRunnable = UisonView::nativeTick;
    private final Runnable stepRunnable = UisonView::nativeStep;

    UisonView(Context c, Bundle extras) {
        super(c);
        setFocusable(true);
        setFocusableInTouchMode(true);
        // Launch parameters (am start --es UISON_STATS 1) — environment variables for Rust.
        StringBuilder env = new StringBuilder();
        String script = null;
        if (extras != null) {
            for (String k : extras.keySet()) {
                env.append(k).append('=').append(extras.get(k)).append('\n');
            }
            // Experiment: software layer (CPU drawing, partial redraw) instead of GPU.
            if ("1".equals(extras.getString("UISON_SOFTWARE"))) {
                setLayerType(LAYER_TYPE_SOFTWARE, null);
            }
            String name = extras.getString("UISON_SCRIPT");
            if (name != null) {
                script = asset(c, name);
            }
        }
        nativeCreate(this, getResources().getDisplayMetrics().density, env.toString(), script);
    }

    /** Check scenario (uison_frame::script) from the APK assets. */
    private static String asset(Context c, String name) {
        try (InputStream in = c.getAssets().open(name)) {
            ByteArrayOutputStream out = new ByteArrayOutputStream();
            byte[] buf = new byte[8192];
            for (int n; (n = in.read(buf)) > 0; ) {
                out.write(buf, 0, n);
            }
            return out.toString("UTF-8");
        } catch (Exception e) {
            return null;
        }
    }

    void appearance(boolean dark) {
        nativeAppearance(dark);
    }

    void paused(boolean p) {
        if (p) {
            removeCallbacks(tickRunnable);
        }
    }

    boolean back() {
        return nativeBack();
    }

    // ---------------------------------------------------------------- from Rust

    /** Show the keyboard (kind — uison_core::Keyboard number, see {@link #inputType}) or hide it (-1). */
    void keyboard(int kind) {
        int type = kind < 0 ? -1 : inputType(kind);
        InputMethodManager imm = (InputMethodManager) getContext().getSystemService(Context.INPUT_METHOD_SERVICE);
        boolean changed = type != input;
        input = type;
        if (type < 0) {
            imm.hideSoftInputFromWindow(getWindowToken(), 0);
            imm.restartInput(this);
        } else {
            requestFocus();
            if (changed) {
                imm.restartInput(this);
            }
            imm.showSoftInput(this, 0);
        }
    }

    /** Image from bytes (PNG, JPEG) of size w×h: roughly downscaled while decoding (inSampleSize), then exactly. */
    static Bitmap decodeImage(byte[] data, int w, int h) {
        BitmapFactory.Options o = new BitmapFactory.Options();
        o.inJustDecodeBounds = true;
        BitmapFactory.decodeByteArray(data, 0, data.length, o);
        int s = 1;
        while (o.outWidth / (s * 2) >= w && o.outHeight / (s * 2) >= h) {
            s *= 2;
        }
        o = new BitmapFactory.Options();
        o.inSampleSize = s;
        Bitmap b = BitmapFactory.decodeByteArray(data, 0, data.length, o);
        if (b == null) {
            return null;
        }
        if (b.getWidth() != w || b.getHeight() != h) {
            Bitmap c = Bitmap.createScaledBitmap(b, w, h, true);
            if (c != b) {
                b.recycle();
            }
            b = c;
        }
        return b;
    }

    private static final Paint IMAGE_PAINT = new Paint(Paint.FILTER_BITMAP_FLAG | Paint.ANTI_ALIAS_FLAG);
    private static final Path CLIP = new Path();
    private static final RectF DST = new RectF();
    private static final Rect SRC = new Rect();

    /** Part of an image (pixels) into a rectangle (points) with rounded corners. */
    static void drawImage(Canvas c, Bitmap b, int sx, int sy, int sw, int sh, float dx, float dy, float dw, float dh, float r) {
        c.save();
        DST.set(dx, dy, dx + dw, dy + dh);
        if (r > 0) {
            float rr = Math.min(r, Math.min(dw, dh) / 2);
            CLIP.reset();
            CLIP.addRoundRect(DST, rr, rr, Path.Direction.CW);
            c.clipPath(CLIP);
        }
        SRC.set(sx, sy, sx + sw, sy + sh);
        c.drawBitmap(b, SRC, DST, IMAGE_PAINT);
        c.restore();
    }

    /** Put text on the clipboard. */
    void copy(String text) {
        ClipboardManager cm = (ClipboardManager) getContext().getSystemService(Context.CLIPBOARD_SERVICE);
        cm.setPrimaryClip(ClipData.newPlainText("text", text));
    }

    /** Text from the clipboard or null. */
    String paste() {
        ClipboardManager cm = (ClipboardManager) getContext().getSystemService(Context.CLIPBOARD_SERVICE);
        ClipData clip = cm.getPrimaryClip();
        if (clip == null || clip.getItemCount() == 0) {
            return null;
        }
        CharSequence t = clip.getItemAt(0).coerceToText(getContext());
        return t == null ? null : t.toString();
    }

    /** Open a link in the app the system chooses. */
    /** The system's file chooser (the activity runs it); false — no activity to run it from. */
    boolean pickFile(String action) {
        Context c = getContext();
        if (!(c instanceof UisonActivity)) {
            return false;
        }
        ((UisonActivity) c).pickFile(action);
        return true;
    }

    /** The chosen file's copy (or "" — cancelled) as a message for the app. */
    void picked(String action, String path) {
        nativeMessage("{\"action\":" + json(action) + ",\"key\":" + json(path) + "}");
    }

    private static String json(String s) {
        StringBuilder b = new StringBuilder("\"");
        for (char ch : s.toCharArray()) {
            if (ch == '"' || ch == '\\') {
                b.append('\\').append(ch);
            } else if (ch < 0x20) {
                b.append(String.format("\\u%04x", (int) ch));
            } else {
                b.append(ch);
            }
        }
        return b.append('"').toString();
    }

    boolean openUrl(String url) {
        try {
            Intent i = new Intent(Intent.ACTION_VIEW, Uri.parse(url));
            i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            getContext().startActivity(i);
            return true;
        } catch (Exception e) {
            return false;
        }
    }

    /** Screen frame for animation (Choreographer). */
    void frame() {
        if (!framePending) {
            framePending = true;
            Choreographer.getInstance().postFrameCallback(this);
        }
    }

    @Override
    public void doFrame(long nanos) {
        framePending = false;
        nativeFrame(nanos);
    }

    /** Timer (caret blinking, app timers). */
    void tick(long ms) {
        removeCallbacks(tickRunnable);
        postDelayed(tickRunnable, ms);
    }

    /** Next step of the check scenario in {@code ms}. */
    void step(long ms) {
        postDelayed(stepRunnable, ms);
    }

    /** Redraw a region (pixels) or everything (w = 0). */
    void damage(int l, int t, int r, int b) {
        if (r <= l) {
            invalidate();
        } else {
            invalidate(l, t, r, b);
        }
    }

    // ---------------------------------------------------------------- events

    @Override
    protected void onSizeChanged(int w, int h, int ow, int oh) {
        nativeResize(w, h);
    }

    @Override
    protected void onDraw(Canvas c) {
        nativeDraw(c);
    }

    @Override
    public WindowInsets onApplyWindowInsets(WindowInsets in) {
        if (Build.VERSION.SDK_INT >= 30) {
            Insets s = in.getInsets(WindowInsets.Type.systemBars() | WindowInsets.Type.displayCutout());
            Insets ime = in.getInsets(WindowInsets.Type.ime());
            nativeInsets(s.top, s.right, s.bottom, s.left, ime.bottom);
        } else {
            // Before Android 11: system insets (with adjustResize — including the keyboard) and the cutout.
            int top = in.getSystemWindowInsetTop(), bottom = in.getSystemWindowInsetBottom();
            int left = in.getSystemWindowInsetLeft(), right = in.getSystemWindowInsetRight();
            int nav = in.getStableInsetBottom();
            if (in.getDisplayCutout() != null) {
                top = Math.max(top, in.getDisplayCutout().getSafeInsetTop());
                left = Math.max(left, in.getDisplayCutout().getSafeInsetLeft());
                right = Math.max(right, in.getDisplayCutout().getSafeInsetRight());
            }
            nativeInsets(top, right, Math.min(bottom, nav), left, bottom > nav ? bottom : 0);
        }
        return in;
    }

    @Override
    public boolean onTouchEvent(MotionEvent e) {
        int a = e.getActionMasked();
        if (a == MotionEvent.ACTION_POINTER_DOWN || a == MotionEvent.ACTION_POINTER_UP) {
            return true; // the second finger doesn't take part
        }
        nativePointer(a, e.getX(), e.getY(), e.getToolType(0), e.getButtonState());
        return true;
    }

    @Override
    public boolean onGenericMotionEvent(MotionEvent e) {
        switch (e.getActionMasked()) {
            case MotionEvent.ACTION_HOVER_MOVE:
            case MotionEvent.ACTION_HOVER_EXIT:
                nativePointer(e.getActionMasked(), e.getX(), e.getY(), e.getToolType(0), 0);
                return true;
            case MotionEvent.ACTION_SCROLL:
                nativeScroll(e.getX(), e.getY(), e.getAxisValue(MotionEvent.AXIS_VSCROLL));
                return true;
        }
        return super.onGenericMotionEvent(e);
    }

    @Override
    public boolean onKeyDown(int code, KeyEvent e) {
        return nativeKey(code, e.getUnicodeChar(), e.getMetaState()) || super.onKeyDown(code, e);
    }

    // ---------------------------------------------------------------- keyboard

    @Override
    public boolean onCheckIsTextEditor() {
        return input >= 0;
    }

    @Override
    public InputConnection onCreateInputConnection(EditorInfo out) {
        if (input < 0) {
            return null;
        }
        out.inputType = input;
        out.imeOptions = EditorInfo.IME_ACTION_DONE | EditorInfo.IME_FLAG_NO_EXTRACT_UI | EditorInfo.IME_FLAG_NO_FULLSCREEN;
        return new Connection(this);
    }

    /**
     * Text from the keyboard → Rust. The word being composed is inserted right away and
     * replaced as typing goes on: the old one is erased with Backspaces, the new one inserted.
     */
    private static final class Connection extends BaseInputConnection {
        private int composing; // characters (code points) of the composed word already in the field

        Connection(View v) {
            super(v, false);
        }

        private void replace(CharSequence text) {
            for (; composing > 0; composing--) {
                nativeKey(KeyEvent.KEYCODE_DEL, 0, 0);
            }
            String s = text.toString();
            if (!s.isEmpty()) {
                nativeText(s);
            }
        }

        @Override
        public boolean setComposingText(CharSequence text, int pos) {
            replace(text);
            composing = text.toString().codePointCount(0, text.length());
            return true;
        }

        @Override
        public boolean commitText(CharSequence text, int pos) {
            replace(text);
            return true;
        }

        @Override
        public boolean finishComposingText() {
            composing = 0;
            return true;
        }

        @Override
        public boolean deleteSurroundingText(int before, int after) {
            for (int i = 0; i < before; i++) {
                nativeKey(KeyEvent.KEYCODE_DEL, 0, 0);
            }
            return true;
        }

        @Override
        public boolean performEditorAction(int action) {
            composing = 0;
            nativeKey(KeyEvent.KEYCODE_ENTER, '\n', 0);
            return true;
        }
    }

    /** EditorInfo.inputType for a uison keyboard type (number as in Rust). */
    static int inputType(int kind) {
        switch (kind) {
            case 1: return InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_EMAIL_ADDRESS;
            case 2: return InputType.TYPE_CLASS_NUMBER;
            case 3: return InputType.TYPE_CLASS_NUMBER | InputType.TYPE_NUMBER_FLAG_DECIMAL;
            case 4: return InputType.TYPE_CLASS_PHONE;
            case 5: return InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_URI;
            case 6: return InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_PASSWORD;
            default: return InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_CAP_SENTENCES;
        }
    }
}
