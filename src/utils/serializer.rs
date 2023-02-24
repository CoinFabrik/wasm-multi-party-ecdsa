use serde::Serialize;
use wasm_bindgen::{JsError, JsValue};

pub fn get_json_serializer() -> serde_wasm_bindgen::Serializer {
    serde_wasm_bindgen::Serializer::json_compatible()
}

pub fn get_json_deserializer(value: JsValue) -> serde_wasm_bindgen::Deserializer {
    serde_wasm_bindgen::Deserializer::from(value)
}

pub fn serialize_str_error_to_js<T: ToString>(error: T) -> JsError {
    JsError::new(&error.to_string())
}

pub fn serialize_serializable_error_to_js<T: serde::Serialize>(error: T) -> JsError {
    JsError::new(&serde_json::to_string(&error).unwrap_or("Unknown error".into()))
}

pub fn serialize_response_to_js(
    res: json_rpc_types::Response<serde_json::Value, serde_json::Value>,
) -> Result<JsValue, JsError> {
    let serializer = get_json_serializer();
    match res.payload {
        Ok(message) => Ok(message.serialize(&serializer)?),
        Err(err) => Err(JsError::new(&serde_json::to_string_pretty(&err)?)), //FIXME
    }
}

pub fn serialize_any_to_js<T: serde::Serialize>(message: T) -> Result<JsValue, JsError> {
    let serializer = get_json_serializer();
    message
        .serialize(&serializer)
        .map_err(serialize_str_error_to_js)
}

pub fn deserialize_any_from_js<'a, T: serde::Deserialize<'a>>(
    value: JsValue,
) -> Result<T, serde_wasm_bindgen::Error> {
    let deserializer = get_json_deserializer(value);
    T::deserialize(deserializer)
}
