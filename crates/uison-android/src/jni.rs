//! Hand-written FFI to JNI: the `JNIEnv` function table (indices from the NDK's `jni.h`) and
//! a thin wrapper over it. Only what is used. Method calls go
//! through the `...A` variants (arguments as a `jvalue` array): C varargs promote
//! float to double, and Rust can't pass them.

#![allow(non_camel_case_types, clippy::missing_safety_doc)]

use std::ffi::{CStr, c_char, c_void};

pub type jobject = *mut c_void;
pub type jclass = jobject;
pub type jstring = jobject;
pub type jmethodID = *mut c_void;
pub type jfieldID = *mut c_void;
pub type jint = i32;
pub type jboolean = u8;

/// `JNIEnv*` — a pointer to a pointer to the function table.
pub type JNIEnv = *const *const c_void;

#[repr(C)]
#[derive(Clone, Copy)]
pub union jvalue {
    pub z: jboolean,
    pub i: jint,
    pub j: i64,
    pub f: f32,
    pub l: jobject,
}

pub fn f(x: f32) -> jvalue {
    jvalue { f: x }
}
pub fn i(x: i32) -> jvalue {
    jvalue { i: x }
}
pub fn long(x: i64) -> jvalue {
    jvalue { j: x }
}
pub fn l(x: jobject) -> jvalue {
    jvalue { l: x }
}

#[repr(C)]
pub struct NativeMethod {
    pub name: *const c_char,
    pub signature: *const c_char,
    pub fn_ptr: *const c_void,
}

// Function indices in the JNINativeInterface table.
const FIND_CLASS: usize = 6;
const EXCEPTION_DESCRIBE: usize = 16;
const EXCEPTION_CLEAR: usize = 17;
const PUSH_LOCAL_FRAME: usize = 19;
const POP_LOCAL_FRAME: usize = 20;
const NEW_GLOBAL_REF: usize = 21;
const DELETE_GLOBAL_REF: usize = 22;
const DELETE_LOCAL_REF: usize = 23;
const NEW_OBJECT_A: usize = 30;
const GET_METHOD_ID: usize = 33;
const CALL_OBJECT_METHOD_A: usize = 36;
const CALL_BOOLEAN_METHOD_A: usize = 39;
const CALL_INT_METHOD_A: usize = 51;
const CALL_FLOAT_METHOD_A: usize = 57;
const CALL_VOID_METHOD_A: usize = 63;
const GET_FIELD_ID: usize = 94;
const GET_INT_FIELD: usize = 100;
const GET_FLOAT_FIELD: usize = 102;
const GET_STATIC_METHOD_ID: usize = 113;
const CALL_STATIC_OBJECT_METHOD_A: usize = 116;
const CALL_STATIC_VOID_METHOD_A: usize = 143;
const GET_STATIC_FIELD_ID: usize = 144;
const GET_STATIC_OBJECT_FIELD: usize = 145;
const NEW_STRING: usize = 163;
const GET_STRING_LENGTH: usize = 164;
const GET_STRING_REGION: usize = 220;
const NEW_BYTE_ARRAY: usize = 176;
const SET_BYTE_ARRAY_REGION: usize = 208;
const REGISTER_NATIVES: usize = 215;
const EXCEPTION_CHECK: usize = 228;

/// `JNIEnv` of the current thread (the main one: all of uison lives on it).
#[derive(Clone, Copy)]
pub struct Env(pub *mut JNIEnv);

impl Env {
    #[inline]
    unsafe fn func<T: Copy>(&self, n: usize) -> T {
        unsafe {
            // JNIEnv* → pointer to the function table.
            let table: *const *const c_void = *self.0;
            std::mem::transmute_copy(&*table.add(n))
        }
    }

    pub unsafe fn find_class(&self, name: &CStr) -> jclass {
        unsafe { self.func::<unsafe extern "C" fn(*mut JNIEnv, *const c_char) -> jclass>(FIND_CLASS)(self.0, name.as_ptr()) }
    }

    pub unsafe fn method(&self, cls: jclass, name: &CStr, sig: &CStr) -> jmethodID {
        let m = unsafe {
            self.func::<unsafe extern "C" fn(*mut JNIEnv, jclass, *const c_char, *const c_char) -> jmethodID>(GET_METHOD_ID)(
                self.0,
                cls,
                name.as_ptr(),
                sig.as_ptr(),
            )
        };
        assert!(!m.is_null(), "no method {name:?} {sig:?}");
        m
    }

    pub unsafe fn static_method(&self, cls: jclass, name: &CStr, sig: &CStr) -> jmethodID {
        let m = unsafe {
            self.func::<unsafe extern "C" fn(*mut JNIEnv, jclass, *const c_char, *const c_char) -> jmethodID>(GET_STATIC_METHOD_ID)(
                self.0,
                cls,
                name.as_ptr(),
                sig.as_ptr(),
            )
        };
        assert!(!m.is_null(), "no static method {name:?} {sig:?}");
        m
    }

    pub unsafe fn field(&self, cls: jclass, name: &CStr, sig: &CStr) -> jfieldID {
        unsafe {
            self.func::<unsafe extern "C" fn(*mut JNIEnv, jclass, *const c_char, *const c_char) -> jfieldID>(GET_FIELD_ID)(
                self.0,
                cls,
                name.as_ptr(),
                sig.as_ptr(),
            )
        }
    }

    /// Static object field (constants like `Paint.Cap.ROUND`) — as a global reference.
    pub unsafe fn static_object(&self, cls: jclass, name: &CStr, sig: &CStr) -> jobject {
        unsafe {
            let id = self.func::<unsafe extern "C" fn(*mut JNIEnv, jclass, *const c_char, *const c_char) -> jfieldID>(
                GET_STATIC_FIELD_ID,
            )(self.0, cls, name.as_ptr(), sig.as_ptr());
            assert!(!id.is_null(), "no static field {name:?}");
            let o = self.func::<unsafe extern "C" fn(*mut JNIEnv, jclass, jfieldID) -> jobject>(GET_STATIC_OBJECT_FIELD)(self.0, cls, id);
            self.global(o)
        }
    }

    pub unsafe fn global(&self, o: jobject) -> jobject {
        unsafe { self.func::<unsafe extern "C" fn(*mut JNIEnv, jobject) -> jobject>(NEW_GLOBAL_REF)(self.0, o) }
    }

    pub unsafe fn delete_global(&self, o: jobject) {
        unsafe { self.func::<unsafe extern "C" fn(*mut JNIEnv, jobject)>(DELETE_GLOBAL_REF)(self.0, o) }
    }

    pub unsafe fn delete_local(&self, o: jobject) {
        unsafe { self.func::<unsafe extern "C" fn(*mut JNIEnv, jobject)>(DELETE_LOCAL_REF)(self.0, o) }
    }

    pub unsafe fn push_frame(&self, capacity: jint) {
        unsafe { self.func::<unsafe extern "C" fn(*mut JNIEnv, jint) -> jint>(PUSH_LOCAL_FRAME)(self.0, capacity) };
    }

    pub unsafe fn pop_frame(&self) {
        unsafe { self.func::<unsafe extern "C" fn(*mut JNIEnv, jobject) -> jobject>(POP_LOCAL_FRAME)(self.0, std::ptr::null_mut()) };
    }

    /// New object — a global reference.
    pub unsafe fn new_object(&self, cls: jclass, ctor: jmethodID, args: &[jvalue]) -> jobject {
        unsafe {
            let o = self.func::<unsafe extern "C" fn(*mut JNIEnv, jclass, jmethodID, *const jvalue) -> jobject>(NEW_OBJECT_A)(
                self.0,
                cls,
                ctor,
                args.as_ptr(),
            );
            let g = self.global(o);
            self.delete_local(o);
            g
        }
    }

    pub unsafe fn call_void(&self, o: jobject, m: jmethodID, args: &[jvalue]) {
        unsafe {
            self.func::<unsafe extern "C" fn(*mut JNIEnv, jobject, jmethodID, *const jvalue)>(CALL_VOID_METHOD_A)(
                self.0,
                o,
                m,
                args.as_ptr(),
            )
        }
    }

    pub unsafe fn call_bool(&self, o: jobject, m: jmethodID, args: &[jvalue]) -> bool {
        unsafe {
            self.func::<unsafe extern "C" fn(*mut JNIEnv, jobject, jmethodID, *const jvalue) -> jboolean>(CALL_BOOLEAN_METHOD_A)(
                self.0,
                o,
                m,
                args.as_ptr(),
            ) != 0
        }
    }

    pub unsafe fn call_int(&self, o: jobject, m: jmethodID, args: &[jvalue]) -> jint {
        unsafe {
            self.func::<unsafe extern "C" fn(*mut JNIEnv, jobject, jmethodID, *const jvalue) -> jint>(CALL_INT_METHOD_A)(
                self.0,
                o,
                m,
                args.as_ptr(),
            )
        }
    }

    pub unsafe fn call_float(&self, o: jobject, m: jmethodID, args: &[jvalue]) -> f32 {
        unsafe {
            self.func::<unsafe extern "C" fn(*mut JNIEnv, jobject, jmethodID, *const jvalue) -> f32>(CALL_FLOAT_METHOD_A)(
                self.0,
                o,
                m,
                args.as_ptr(),
            )
        }
    }

    pub unsafe fn call_object(&self, o: jobject, m: jmethodID, args: &[jvalue]) -> jobject {
        unsafe {
            self.func::<unsafe extern "C" fn(*mut JNIEnv, jobject, jmethodID, *const jvalue) -> jobject>(CALL_OBJECT_METHOD_A)(
                self.0,
                o,
                m,
                args.as_ptr(),
            )
        }
    }

    pub unsafe fn call_static_object(&self, cls: jclass, m: jmethodID, args: &[jvalue]) -> jobject {
        unsafe {
            self.func::<unsafe extern "C" fn(*mut JNIEnv, jclass, jmethodID, *const jvalue) -> jobject>(CALL_STATIC_OBJECT_METHOD_A)(
                self.0,
                cls,
                m,
                args.as_ptr(),
            )
        }
    }

    pub unsafe fn call_static_void(&self, cls: jclass, m: jmethodID, args: &[jvalue]) {
        unsafe {
            self.func::<unsafe extern "C" fn(*mut JNIEnv, jclass, jmethodID, *const jvalue)>(CALL_STATIC_VOID_METHOD_A)(
                self.0,
                cls,
                m,
                args.as_ptr(),
            )
        }
    }

    /// `byte[]` with a copy of `b` (local reference).
    pub unsafe fn byte_array(&self, b: &[u8]) -> jobject {
        unsafe {
            let a = self.func::<unsafe extern "C" fn(*mut JNIEnv, jint) -> jobject>(NEW_BYTE_ARRAY)(self.0, b.len() as jint);
            if !a.is_null() {
                self.func::<unsafe extern "C" fn(*mut JNIEnv, jobject, jint, jint, *const u8)>(SET_BYTE_ARRAY_REGION)(
                    self.0,
                    a,
                    0,
                    b.len() as jint,
                    b.as_ptr(),
                );
            }
            a
        }
    }

    pub unsafe fn int_field(&self, o: jobject, f: jfieldID) -> jint {
        unsafe { self.func::<unsafe extern "C" fn(*mut JNIEnv, jobject, jfieldID) -> jint>(GET_INT_FIELD)(self.0, o, f) }
    }

    pub unsafe fn float_field(&self, o: jobject, f: jfieldID) -> f32 {
        unsafe { self.func::<unsafe extern "C" fn(*mut JNIEnv, jobject, jfieldID) -> f32>(GET_FLOAT_FIELD)(self.0, o, f) }
    }

    /// Java string from UTF-8 (via UTF-16: JNI's "modified UTF-8" can't do emoji).
    pub unsafe fn new_string(&self, s: &str) -> jstring {
        let u: Vec<u16> = s.encode_utf16().collect();
        unsafe {
            self.func::<unsafe extern "C" fn(*mut JNIEnv, *const u16, jint) -> jstring>(NEW_STRING)(self.0, u.as_ptr(), u.len() as jint)
        }
    }

    pub unsafe fn string(&self, s: jstring) -> String {
        if s.is_null() {
            return String::new();
        }
        unsafe {
            let n = self.func::<unsafe extern "C" fn(*mut JNIEnv, jstring) -> jint>(GET_STRING_LENGTH)(self.0, s);
            let mut buf = vec![0u16; n as usize];
            self.func::<unsafe extern "C" fn(*mut JNIEnv, jstring, jint, jint, *mut u16)>(GET_STRING_REGION)(
                self.0,
                s,
                0,
                n,
                buf.as_mut_ptr(),
            );
            String::from_utf16_lossy(&buf)
        }
    }

    pub unsafe fn register(&self, cls: jclass, methods: &[NativeMethod]) {
        let r = unsafe {
            self.func::<unsafe extern "C" fn(*mut JNIEnv, jclass, *const NativeMethod, jint) -> jint>(REGISTER_NATIVES)(
                self.0,
                cls,
                methods.as_ptr(),
                methods.len() as jint,
            )
        };
        assert!(r == 0, "RegisterNatives failed");
    }

    /// A Java exception after a call — log it and clear it (otherwise the next JNI call crashes).
    pub unsafe fn check(&self) -> bool {
        unsafe {
            let pending = self.func::<unsafe extern "C" fn(*mut JNIEnv) -> jboolean>(EXCEPTION_CHECK)(self.0) != 0;
            if pending {
                self.func::<unsafe extern "C" fn(*mut JNIEnv)>(EXCEPTION_DESCRIBE)(self.0);
                self.func::<unsafe extern "C" fn(*mut JNIEnv)>(EXCEPTION_CLEAR)(self.0);
            }
            pending
        }
    }
}
