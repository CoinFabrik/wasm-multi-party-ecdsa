use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const SESSION_KIND: &'static str = r#"type SessionKind = "keygen" | "sign";"#;

#[wasm_bindgen(typescript_custom_section)]
const GROUP: &'static str = r#"
interface Group {
    id: string;
    params: {
        n: number;
        t: number;
    }
}"#;

#[wasm_bindgen(typescript_custom_section)]
const SESSION: &'static str = r#"
interface Session {
    id: string;
    kind: SessionKind;
    value: any | null;
}"#;

#[wasm_bindgen(typescript_custom_section)]
const GROUP_CREATE_RESPONSE: &'static str = r#"
interface GroupCreateResponse {
    group: Group;
}"#;

#[wasm_bindgen(typescript_custom_section)]
const GROUP_JOIN_RESPONSE: &'static str = r#"
interface GroupJoinResponse {
    group: Group;
}"#;

#[wasm_bindgen(typescript_custom_section)]
const SESSION_CREATE_RESPONSE: &'static str = r#"
interface SessionCreateResponse {
    session: Session;
}"#;

#[wasm_bindgen(typescript_custom_section)]
const SESSION_SIGNUP_RESPONSE: &'static str = r#"
interface SessionSignupResponse {
    session: Session;
    partyNumber: number;
}"#;

#[wasm_bindgen(typescript_custom_section)]
const SESSION_LOGIN_RESPONSE: &'static str = r#"
interface SessionLoginResponse {
    session: Session;
}"#;

#[wasm_bindgen(typescript_custom_section)]
const KEYGEN_RESPONSE: &'static str = r#"
interface KeygenResponse {
    localKey: any;
    publicKey: string;
}"#;

#[wasm_bindgen(typescript_custom_section)]
const SIGN_RESPONSE: &'static str = r#"
interface SignResponse {
    r: string;
    s: string;
    recid: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "SessionKind")]
    pub type SessionKind;
    #[wasm_bindgen(typescript_type = "GroupCreateResponse")]
    pub type GroupCreateResponse;
    #[wasm_bindgen(typescript_type = "GroupJoinResponse")]
    pub type GroupJoinResponse;
    #[wasm_bindgen(typescript_type = "SessionCreateResponse")]
    pub type SessionCreateResponse;
    #[wasm_bindgen(typescript_type = "SessionSignupResponse")]
    pub type SessionSignupResponse;
    #[wasm_bindgen(typescript_type = "SessionLoginResponse")]
    pub type SessionLoginResponse;
    #[wasm_bindgen(typescript_type = "KeygenResponse")]
    pub type KeygenResponse;
    #[wasm_bindgen(typescript_type = "SignResponse")]
    pub type SignResponse;
}
