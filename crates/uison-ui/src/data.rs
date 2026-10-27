//! Data model: a JSON tree accessed by paths like `form.email`, `todos.3.done`.

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};

use crate::Json;

/// Anything that can be put into the model or into a node property.
pub trait IntoJson {
    fn into_json(self) -> Json;
}

impl IntoJson for Json {
    fn into_json(self) -> Json {
        self
    }
}

impl IntoJson for &Json {
    fn into_json(self) -> Json {
        self.clone()
    }
}

impl IntoJson for &str {
    fn into_json(self) -> Json {
        Json::String(Cow::Owned(self.to_owned()))
    }
}

impl IntoJson for String {
    fn into_json(self) -> Json {
        Json::String(Cow::Owned(self))
    }
}

impl IntoJson for &String {
    fn into_json(self) -> Json {
        self.as_str().into_json()
    }
}

macro_rules! into_json_via_from {
    ($($t:ty),*) => {$(
        impl IntoJson for $t {
            fn into_json(self) -> Json {
                Json::from(self)
            }
        }
    )*};
}

into_json_via_from!(bool, f64, f32, i32, i64, u32, usize);

macro_rules! into_json_number {
    ($($t:ty),*) => {$(
        impl IntoJson for $t {
            fn into_json(self) -> Json {
                Json::Number(self as f64)
            }
        }
    )*};
}

into_json_number!(i8, i16, u8, u16, u64, isize);

impl<T: IntoJson> IntoJson for Vec<T> {
    fn into_json(self) -> Json {
        Json::Array(self.into_iter().map(IntoJson::into_json).collect())
    }
}

impl<T: IntoJson, const N: usize> IntoJson for [T; N] {
    fn into_json(self) -> Json {
        Json::Array(self.into_iter().map(IntoJson::into_json).collect())
    }
}

impl<T: IntoJson> IntoJson for Option<T> {
    fn into_json(self) -> Json {
        self.map_or(Json::Null, IntoJson::into_json)
    }
}

impl<T: IntoJson> IntoJson for HashMap<String, T> {
    fn into_json(self) -> Json {
        // HashMap order is random — keys sorted alphabetically so the data (and the frame) are stable.
        let mut v: Vec<_> = self.into_iter().collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        Json::Object(v.into_iter().map(|(k, v)| (Cow::Owned(k), v.into_json())).collect())
    }
}

impl<T: IntoJson> IntoJson for BTreeMap<String, T> {
    fn into_json(self) -> Json {
        Json::Object(self.into_iter().map(|(k, v)| (Cow::Owned(k), v.into_json())).collect())
    }
}

/// Counterpart of [`IntoJson`]: model value → Rust type (`cx.data.read::<Form>("form")`). `None` — wrong type.
pub trait FromJson: Sized {
    fn from_json(v: &Json) -> Option<Self>;
}

impl FromJson for Json {
    fn from_json(v: &Json) -> Option<Json> {
        Some(v.clone())
    }
}

/// `true`/`false`; a number — non-zero (data from C, where bool is written as 0/1).
impl FromJson for bool {
    fn from_json(v: &Json) -> Option<bool> {
        match v {
            Json::Bool(b) => Some(*b),
            Json::Number(n) => Some(*n != 0.0),
            _ => None,
        }
    }
}

/// A string; a number and `bool` — their textual form (like `Data::str` in C: `uison_data_str`).
impl FromJson for String {
    fn from_json(v: &Json) -> Option<String> {
        match v {
            Json::String(s) => Some(s.to_string()),
            Json::Number(_) | Json::Bool(_) => Some(v.to_json()),
            _ => None,
        }
    }
}

macro_rules! from_json_number {
    ($($t:ty),*) => {$(
        impl FromJson for $t {
            fn from_json(v: &Json) -> Option<$t> {
                v.as_f64().map(|n| n as $t)
            }
        }
    )*};
}

from_json_number!(f64, f32, i8, i16, i32, i64, u8, u16, u32, u64, isize, usize);

/// `null` — `Some(None)`; otherwise a `T` value.
impl<T: FromJson> FromJson for Option<T> {
    fn from_json(v: &Json) -> Option<Option<T>> {
        match v {
            Json::Null => Some(None),
            v => T::from_json(v).map(Some),
        }
    }
}

/// Array: if any element has the wrong type — `None`.
impl<T: FromJson> FromJson for Vec<T> {
    fn from_json(v: &Json) -> Option<Vec<T>> {
        v.as_array()?.iter().map(T::from_json).collect()
    }
}

impl<T: FromJson> FromJson for HashMap<String, T> {
    fn from_json(v: &Json) -> Option<HashMap<String, T>> {
        match v {
            Json::Object(o) => o.iter().map(|(k, v)| Some((k.to_string(), T::from_json(v)?))).collect(),
            _ => None,
        }
    }
}

impl<T: FromJson> FromJson for BTreeMap<String, T> {
    fn from_json(v: &Json) -> Option<BTreeMap<String, T>> {
        match v {
            Json::Object(o) => o.iter().map(|(k, v)| Some((k.to_string(), T::from_json(v)?))).collect(),
            _ => None,
        }
    }
}

/// Object from pairs: `obj([("id", 1.into_json()), ("title", "Buy bread".into_json())])`.
pub fn obj<const N: usize>(fields: [(&str, Json); N]) -> Json {
    Json::Object(fields.into_iter().map(|(k, v)| (Cow::Owned(k.to_owned()), v)).collect())
}

/// Data model. Every change increments `version` — that is how the runtime
/// knows the frame must be rebuilt.
pub struct Data {
    root: Json,
    version: u64,
}

impl Default for Data {
    fn default() -> Self {
        Data::new(Json::object())
    }
}

impl Data {
    pub fn new(root: Json) -> Data {
        Data { root, version: 0 }
    }

    pub fn root(&self) -> &Json {
        &self.root
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn get(&self, path: &str) -> Option<&Json> {
        segments(path).try_fold(&self.root, |v, seg| step(v, seg))
    }

    /// Mutable access. The version is bumped right away: we assume the value will be changed.
    pub fn get_mut(&mut self, path: &str) -> Option<&mut Json> {
        self.version += 1;
        segments(path).try_fold(&mut self.root, |v, seg| step_mut(v, seg))
    }

    /// Write a value; missing objects along the path are created.
    /// An array index must exist (or equal the length — then it appends).
    pub fn set(&mut self, path: &str, value: impl IntoJson) {
        let value = value.into_json();
        if self.get(path) == Some(&value) {
            return;
        }
        self.version += 1;
        let mut cur = &mut self.root;
        let segs: Vec<&str> = segments(path).collect();
        if segs.is_empty() {
            self.root = value;
            return;
        }
        for (i, seg) in segs.iter().enumerate() {
            let last = i + 1 == segs.len();
            if !matches!(cur, Json::Object(_) | Json::Array(_)) {
                *cur = Json::object();
            }
            if let Json::Array(a) = cur {
                let Ok(idx) = seg.parse::<usize>() else { return };
                if idx == a.len() {
                    a.push(Json::Null);
                }
                let Some(slot) = a.get_mut(idx) else { return };
                if last {
                    *slot = value;
                    return;
                }
                cur = slot;
            } else {
                if cur.get(seg).is_none() {
                    cur.set(seg, if last { Json::Null } else { Json::object() });
                }
                let slot = cur.get_mut(seg).expect("just inserted");
                if last {
                    *slot = value;
                    return;
                }
                cur = slot;
            }
        }
    }

    /// Change a value with a function of the old one.
    pub fn update<T: IntoJson>(&mut self, path: &str, f: impl FnOnce(&Json) -> T) {
        let old = self.get(path).cloned().unwrap_or(Json::Null);
        self.set(path, f(&old));
    }

    /// Value at a path as a Rust type: `cx.data.read::<Vec<Todo>>("todos")`. Missing or wrong type — `None`.
    pub fn read<T: FromJson>(&self, path: &str) -> Option<T> {
        self.get(path).and_then(T::from_json)
    }

    /// Change a value as a Rust type and write it back: `cx.data.edit("form", |f: &mut Form| f.name.clear())`.
    /// No value (or the wrong type) — start from `T::default()`.
    pub fn edit<T: FromJson + IntoJson + Default>(&mut self, path: &str, f: impl FnOnce(&mut T)) {
        let mut v = self.read::<T>(path).unwrap_or_default();
        f(&mut v);
        self.set(path, v);
    }

    pub fn str(&self, path: &str) -> &str {
        self.get(path).and_then(Json::as_str).unwrap_or("")
    }

    pub fn num(&self, path: &str) -> f64 {
        self.get(path).and_then(Json::as_f64).unwrap_or(0.0)
    }

    /// "Truthiness" of a value (see [`Json::truthy`]).
    pub fn bool(&self, path: &str) -> bool {
        self.get(path).is_some_and(Json::truthy)
    }

    pub fn len(&self, path: &str) -> usize {
        self.get(path).and_then(Json::as_array).map_or(0, <[Json]>::len)
    }

    pub fn push(&mut self, path: &str, value: impl IntoJson) {
        let value = value.into_json();
        if self.get(path).and_then(Json::as_array).is_none() {
            self.set(path, Json::Array(Vec::new()));
        }
        if let Some(a) = self.get_mut(path).and_then(Json::as_array_mut) {
            a.push(value);
        }
    }

    pub fn remove(&mut self, path: &str, index: usize) -> Option<Json> {
        let a = self.get_mut(path)?.as_array_mut()?;
        (index < a.len()).then(|| a.remove(index))
    }

    /// Keep only the array elements for which `keep` returned true.
    pub fn retain(&mut self, path: &str, keep: impl FnMut(&Json) -> bool) {
        if let Some(a) = self.get_mut(path).and_then(Json::as_array_mut) {
            a.retain(keep);
        }
    }
}

pub(crate) fn segments(path: &str) -> impl Iterator<Item = &str> {
    path.split('.').filter(|s| !s.is_empty())
}

/// A path step: an object field or an array index.
pub(crate) fn step<'v>(v: &'v Json, seg: &str) -> Option<&'v Json> {
    match v {
        Json::Object(_) => v.get(seg),
        Json::Array(a) => a.get(seg.parse::<usize>().ok()?),
        _ => None,
    }
}

fn step_mut<'v>(v: &'v mut Json, seg: &str) -> Option<&'v mut Json> {
    match v {
        Json::Object(_) => v.get_mut(seg),
        Json::Array(a) => a.get_mut(seg.parse::<usize>().ok()?),
        _ => None,
    }
}
