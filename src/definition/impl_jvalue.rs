use super::{JValue, VKind};
impl JValue {
  pub fn is_lit(&self) -> bool {
    match self {
      JValue::Null => true,
      JValue::Bool(v) => matches!(v, VKind::Lit(_)),
      JValue::Int(v) => matches!(v, VKind::Lit(_)),
      JValue::Float(v) => matches!(v, VKind::Lit(_)),
      JValue::String(v) => matches!(v, VKind::Lit(_)),
      JValue::Array(v) => matches!(v, VKind::Lit(_)),
      JValue::Object(v) => matches!(v, VKind::Lit(_)),
      JValue::Function(v) => matches!(v, VKind::Lit(_)),
    }
  }
}
