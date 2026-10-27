//! Java classes and methods uison calls: looked up once in JNI_OnLoad.
//! `jmethodID`s and global references are valid for the whole process.

use std::ffi::CStr;
use std::sync::OnceLock;

use crate::jni::*;

pub struct J {
    // android.graphics.Canvas
    pub draw_rect: jmethodID,
    pub draw_round_rect: jmethodID,
    pub draw_circle: jmethodID,
    pub draw_line: jmethodID,
    pub draw_path: jmethodID,
    pub draw_text: jmethodID,
    pub save: jmethodID,
    pub restore: jmethodID,
    pub clip_rect: jmethodID,
    pub scale: jmethodID,
    pub get_clip_bounds: jmethodID,
    // android.graphics.Rect
    pub rect_cls: jclass,
    pub rect_new: jmethodID,
    pub rect_left: jfieldID,
    pub rect_top: jfieldID,
    pub rect_right: jfieldID,
    pub rect_bottom: jfieldID,
    // android.graphics.Paint
    pub paint_cls: jclass,
    pub paint_new: jmethodID,
    pub set_color: jmethodID,
    pub set_stroke_width: jmethodID,
    pub set_stroke_cap: jmethodID,
    pub set_stroke_join: jmethodID,
    pub set_style: jmethodID,
    pub set_text_size: jmethodID,
    pub set_typeface: jmethodID,
    pub measure_text: jmethodID,
    pub get_font_metrics: jmethodID,
    pub cap_round: jobject,
    pub cap_butt: jobject,
    pub join_round: jobject,
    pub style_stroke: jobject,
    // android.graphics.Paint$FontMetrics
    pub fm_cls: jclass,
    pub fm_new: jmethodID,
    pub fm_ascent: jfieldID,
    pub fm_descent: jfieldID,
    pub fm_leading: jfieldID,
    // android.graphics.Path
    pub path_cls: jclass,
    pub path_new: jmethodID,
    pub path_reset: jmethodID,
    pub path_move_to: jmethodID,
    pub path_line_to: jmethodID,
    pub path_close: jmethodID,
    // android.graphics.Typeface
    pub typeface_cls: jclass,
    pub typeface_default: jobject,
    pub typeface_bold: jobject,
    pub typeface_create: jmethodID,
    // dev.uison.UisonView — callbacks
    pub view_cls: jclass,
    pub view_keyboard: jmethodID,
    pub view_frame: jmethodID,
    pub view_tick: jmethodID,
    pub view_step: jmethodID,
    pub view_damage: jmethodID,
    pub view_copy: jmethodID,
    pub view_paste: jmethodID,
    pub view_open_url: jmethodID,
    pub view_pick_file: jmethodID,
    pub view_decode_image: jmethodID,
    pub view_draw_image: jmethodID,
}

// Only IDs and global references — they can be read from any thread.
unsafe impl Send for J {}
unsafe impl Sync for J {}

static CELL: OnceLock<J> = OnceLock::new();

pub fn j() -> &'static J {
    CELL.get().expect("uison: Java classes not loaded")
}

unsafe fn class(env: Env, name: &CStr) -> jclass {
    unsafe {
        let c = env.find_class(name);
        assert!(!c.is_null(), "no class {name:?}");
        env.global(c)
    }
}

/// Find everything needed. Called from JNI_OnLoad (the class dev.uison.UisonView is visible to the app's class loader).
pub unsafe fn init(env: Env) -> jclass {
    unsafe {
        let canvas = class(env, c"android/graphics/Canvas");
        let rect = class(env, c"android/graphics/Rect");
        let paint = class(env, c"android/graphics/Paint");
        let cap = class(env, c"android/graphics/Paint$Cap");
        let join = class(env, c"android/graphics/Paint$Join");
        let style = class(env, c"android/graphics/Paint$Style");
        let fm = class(env, c"android/graphics/Paint$FontMetrics");
        let path = class(env, c"android/graphics/Path");
        let tf = class(env, c"android/graphics/Typeface");
        let view = class(env, c"dev/uison/UisonView");
        let j = J {
            draw_rect: env.method(canvas, c"drawRect", c"(FFFFLandroid/graphics/Paint;)V"),
            draw_round_rect: env.method(canvas, c"drawRoundRect", c"(FFFFFFLandroid/graphics/Paint;)V"),
            draw_circle: env.method(canvas, c"drawCircle", c"(FFFLandroid/graphics/Paint;)V"),
            draw_line: env.method(canvas, c"drawLine", c"(FFFFLandroid/graphics/Paint;)V"),
            draw_path: env.method(canvas, c"drawPath", c"(Landroid/graphics/Path;Landroid/graphics/Paint;)V"),
            draw_text: env.method(canvas, c"drawText", c"(Ljava/lang/String;FFLandroid/graphics/Paint;)V"),
            save: env.method(canvas, c"save", c"()I"),
            restore: env.method(canvas, c"restore", c"()V"),
            clip_rect: env.method(canvas, c"clipRect", c"(FFFF)Z"),
            scale: env.method(canvas, c"scale", c"(FF)V"),
            get_clip_bounds: env.method(canvas, c"getClipBounds", c"(Landroid/graphics/Rect;)Z"),
            rect_cls: rect,
            rect_new: env.method(rect, c"<init>", c"()V"),
            rect_left: env.field(rect, c"left", c"I"),
            rect_top: env.field(rect, c"top", c"I"),
            rect_right: env.field(rect, c"right", c"I"),
            rect_bottom: env.field(rect, c"bottom", c"I"),
            paint_cls: paint,
            paint_new: env.method(paint, c"<init>", c"(I)V"),
            set_color: env.method(paint, c"setColor", c"(I)V"),
            set_stroke_width: env.method(paint, c"setStrokeWidth", c"(F)V"),
            set_stroke_cap: env.method(paint, c"setStrokeCap", c"(Landroid/graphics/Paint$Cap;)V"),
            set_stroke_join: env.method(paint, c"setStrokeJoin", c"(Landroid/graphics/Paint$Join;)V"),
            set_style: env.method(paint, c"setStyle", c"(Landroid/graphics/Paint$Style;)V"),
            set_text_size: env.method(paint, c"setTextSize", c"(F)V"),
            set_typeface: env.method(paint, c"setTypeface", c"(Landroid/graphics/Typeface;)Landroid/graphics/Typeface;"),
            measure_text: env.method(paint, c"measureText", c"(Ljava/lang/String;)F"),
            get_font_metrics: env.method(paint, c"getFontMetrics", c"(Landroid/graphics/Paint$FontMetrics;)F"),
            cap_round: env.static_object(cap, c"ROUND", c"Landroid/graphics/Paint$Cap;"),
            cap_butt: env.static_object(cap, c"BUTT", c"Landroid/graphics/Paint$Cap;"),
            join_round: env.static_object(join, c"ROUND", c"Landroid/graphics/Paint$Join;"),
            style_stroke: env.static_object(style, c"STROKE", c"Landroid/graphics/Paint$Style;"),
            fm_cls: fm,
            fm_new: env.method(fm, c"<init>", c"()V"),
            fm_ascent: env.field(fm, c"ascent", c"F"),
            fm_descent: env.field(fm, c"descent", c"F"),
            fm_leading: env.field(fm, c"leading", c"F"),
            path_cls: path,
            path_new: env.method(path, c"<init>", c"()V"),
            path_reset: env.method(path, c"reset", c"()V"),
            path_move_to: env.method(path, c"moveTo", c"(FF)V"),
            path_line_to: env.method(path, c"lineTo", c"(FF)V"),
            path_close: env.method(path, c"close", c"()V"),
            typeface_cls: tf,
            typeface_default: env.static_object(tf, c"DEFAULT", c"Landroid/graphics/Typeface;"),
            typeface_bold: env.static_object(tf, c"DEFAULT_BOLD", c"Landroid/graphics/Typeface;"),
            typeface_create: env.static_method(tf, c"create", c"(Ljava/lang/String;I)Landroid/graphics/Typeface;"),
            view_cls: view,
            view_keyboard: env.method(view, c"keyboard", c"(I)V"),
            view_frame: env.method(view, c"frame", c"()V"),
            view_tick: env.method(view, c"tick", c"(J)V"),
            view_step: env.method(view, c"step", c"(J)V"),
            view_damage: env.method(view, c"damage", c"(IIII)V"),
            view_copy: env.method(view, c"copy", c"(Ljava/lang/String;)V"),
            view_paste: env.method(view, c"paste", c"()Ljava/lang/String;"),
            view_open_url: env.method(view, c"openUrl", c"(Ljava/lang/String;)Z"),
            view_pick_file: env.method(view, c"pickFile", c"(Ljava/lang/String;)Z"),
            view_decode_image: env.static_method(view, c"decodeImage", c"([BII)Landroid/graphics/Bitmap;"),
            view_draw_image: env.static_method(
                view,
                c"drawImage",
                c"(Landroid/graphics/Canvas;Landroid/graphics/Bitmap;IIIIFFFFF)V",
            ),
        };
        let cls = j.view_cls;
        let _ = CELL.set(j);
        cls
    }
}
