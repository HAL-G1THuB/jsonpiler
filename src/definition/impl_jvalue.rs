use super::{JValue, VKind};
impl JValue {
  pub fn is_lit(&self) -> bool {
    matches!(
      self,
      JValue::Null
        | JValue::Bool(VKind::Lit(_))
        | JValue::Int(VKind::Lit(_))
        | JValue::Float(VKind::Lit(_))
        | JValue::String(VKind::Lit(_))
        | JValue::Array(VKind::Lit(_))
        | JValue::Object(VKind::Lit(_))
    )
    //function isn't literal
  }
}
