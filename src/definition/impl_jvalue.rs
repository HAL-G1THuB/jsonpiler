use super::JValue;
impl JValue {
  pub fn is_lit(&self) -> bool {
    matches!(
      self,
      JValue::Null
        | JValue::Bool(_)
        | JValue::Int(_)
        | JValue::Float(_)
        | JValue::String(_)
        | JValue::Array(_)
        | JValue::Object(_)
    )
  }
}
