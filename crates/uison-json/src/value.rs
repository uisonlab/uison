use std::borrow::Cow;

use crate::{Kind, Parser, Result};

/// Value tree. Strings borrow from the source text when possible.
#[derive(Debug, Clone, PartialEq)]
pub enum Value<'a> {
    Null,
    Bool(bool),
    Number(f64),
    String(Cow<'a, str>),
    Array(Vec<Value<'a>>),
    /// Key order is preserved; lookup is linear (objects are usually small).
    Object(Vec<(Cow<'a, str>, Value<'a>)>),
}

impl<'a> Value<'a> {
    pub fn get(&self, key: &str) -> Option<&Value<'a>> {
        match self {
            Value::Object(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        if let Value::Number(n) = self { Some(*n) } else { None }
    }

    pub fn as_bool(&self) -> Option<bool> {
        if let Value::Bool(b) = self { Some(*b) } else { None }
    }

    pub fn as_str(&self) -> Option<&str> {
        if let Value::String(s) = self { Some(s) } else { None }
    }

    pub fn as_array(&self) -> Option<&[Value<'a>]> {
        if let Value::Array(a) = self { Some(a) } else { None }
    }

    pub fn as_object(&self) -> Option<&[(Cow<'a, str>, Value<'a>)]> {
        if let Value::Object(o) = self { Some(o) } else { None }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Truthiness for conditions: `null`, `false`, `0`, `""`, `[]` are false.
    pub fn truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Number(n) => *n != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Object(_) => true,
        }
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value<'a>> {
        match self {
            Value::Object(fields) => fields.iter_mut().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Value<'a>>> {
        if let Value::Array(a) = self { Some(a) } else { None }
    }

    pub fn as_object_mut(&mut self) -> Option<&mut Vec<(Cow<'a, str>, Value<'a>)>> {
        if let Value::Object(o) = self { Some(o) } else { None }
    }

    /// Set an object field (replace or append). Does nothing on a non-object.
    pub fn set(&mut self, key: &str, v: impl Into<Value<'a>>) {
        if let Value::Object(fields) = self {
            let v = v.into();
            match fields.iter_mut().find(|(k, _)| k == key) {
                Some((_, slot)) => *slot = v,
                None => fields.push((Cow::Owned(key.to_owned()), v)),
            }
        }
    }

    pub fn remove(&mut self, key: &str) -> Option<Value<'a>> {
        let fields = self.as_object_mut()?;
        let i = fields.iter().position(|(k, _)| k == key)?;
        Some(fields.remove(i).1)
    }

    /// Detach from the source text.
    pub fn into_owned(self) -> Value<'static> {
        match self {
            Value::Null => Value::Null,
            Value::Bool(b) => Value::Bool(b),
            Value::Number(n) => Value::Number(n),
            Value::String(s) => Value::String(Cow::Owned(s.into_owned())),
            Value::Array(a) => Value::Array(a.into_iter().map(Value::into_owned).collect()),
            Value::Object(o) => {
                Value::Object(o.into_iter().map(|(k, v)| (Cow::Owned(k.into_owned()), v.into_owned())).collect())
            }
        }
    }

    pub fn object() -> Value<'a> {
        Value::Object(Vec::new())
    }
}

impl From<bool> for Value<'_> {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<f64> for Value<'_> {
    fn from(v: f64) -> Self {
        Value::Number(v)
    }
}

impl From<f32> for Value<'_> {
    fn from(v: f32) -> Self {
        Value::Number(v as f64)
    }
}

impl From<i32> for Value<'_> {
    fn from(v: i32) -> Self {
        Value::Number(v as f64)
    }
}

impl From<i64> for Value<'_> {
    fn from(v: i64) -> Self {
        Value::Number(v as f64)
    }
}

impl From<u32> for Value<'_> {
    fn from(v: u32) -> Self {
        Value::Number(v as f64)
    }
}

impl From<usize> for Value<'_> {
    fn from(v: usize) -> Self {
        Value::Number(v as f64)
    }
}

impl From<String> for Value<'_> {
    fn from(v: String) -> Self {
        Value::String(Cow::Owned(v))
    }
}

impl<'a> From<&'a str> for Value<'a> {
    fn from(v: &'a str) -> Self {
        Value::String(Cow::Borrowed(v))
    }
}

impl<'a, T: Into<Value<'a>>> From<Vec<T>> for Value<'a> {
    fn from(v: Vec<T>) -> Self {
        Value::Array(v.into_iter().map(Into::into).collect())
    }
}

impl<'a, T: Into<Value<'a>>> From<Option<T>> for Value<'a> {
    fn from(v: Option<T>) -> Self {
        v.map_or(Value::Null, Into::into)
    }
}

impl<'a> Parser<'a> {
    pub fn value(&mut self) -> Result<Value<'a>> {
        Ok(match self.peek()? {
            Kind::Null => {
                self.null()?;
                Value::Null
            }
            Kind::Bool => Value::Bool(self.boolean()?),
            Kind::Number => Value::Number(self.number()?),
            Kind::String => Value::String(self.string()?),
            Kind::Array => {
                self.begin_array()?;
                let mut items = Vec::new();
                while self.next_item()? {
                    items.push(self.value()?);
                }
                Value::Array(items)
            }
            Kind::Object => {
                self.begin_object()?;
                let mut fields = Vec::new();
                while let Some(k) = self.next_key()? {
                    fields.push((k, self.value()?));
                }
                Value::Object(fields)
            }
        })
    }
}

/// Parse a whole document.
pub fn parse(src: &str) -> Result<Value<'_>> {
    let mut p = Parser::new(src);
    let v = p.value()?;
    p.finish()?;
    Ok(v)
}
