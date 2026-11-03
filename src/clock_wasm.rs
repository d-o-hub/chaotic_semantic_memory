//! WASM millisecond source for [`super::Clock`].

pub(super) fn now_ms() -> u64 {
    js_sys::Date::now() as u64
}
