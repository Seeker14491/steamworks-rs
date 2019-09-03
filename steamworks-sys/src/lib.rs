#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
#![allow(clippy::all)]

#![allow(deprecated, invalid_value)] // Silence warning generated as of bindgen 0.51

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
