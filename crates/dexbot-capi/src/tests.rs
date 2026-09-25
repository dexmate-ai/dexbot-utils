use super::*;
#[test]
fn errors_and_borrowed_values() {
    unsafe {
        assert!(dexbot_resolve(ptr::null(), false, ptr::null()).is_null());
        assert!(!CStr::from_ptr(dexbot_last_error()).to_bytes().is_empty());
        let source = CString::new("vega_1p").unwrap();
        let root = dexbot_resolve(source.as_ptr(), false, ptr::null());
        assert!(!root.is_null());
        let key = CString::new("profile_name").unwrap();
        let child = dexbot_get(root, key.as_ptr());
        let mut len = 0;
        let data = dexbot_string(child, &mut len);
        assert_eq!(
            std::slice::from_raw_parts(data.cast::<u8>(), len),
            b"vega_1p"
        );
        assert!(!dexbot_number(child, ptr::null_mut()));
        assert!(dexbot_at(root, usize::MAX).is_null());
        dexbot_document_free(root);
        dexbot_document_free(ptr::null_mut());
    }
}
#[test]
fn invalid_utf8_and_failed_urdf_are_reported() {
    unsafe {
        assert_eq!(dexbot_abi_version(), 1);
        let invalid = [0xffu8, 0];
        assert!(dexbot_resolve(invalid.as_ptr().cast(), false, ptr::null()).is_null());
        assert!(CStr::from_ptr(dexbot_last_error())
            .to_str()
            .unwrap()
            .contains("UTF-8"));
        let xml = CString::new("broken xml").unwrap();
        assert!(dexbot_parse_urdf(xml.as_ptr()).is_null());
        assert_eq!(dexbot_type(ptr::null()), -1);
    }
}
#[test]
fn panic_is_contained() {
    assert_eq!(guard(42, || -> Result<i32, String> { panic!("test") }), 42);
}

#[test]
fn typed_values_and_owned_results_preserve_their_contracts() {
    let root = serde_json::json!({"values": [null, true, 12.5, "a\u{0}β"]});
    unsafe {
        let values = dexbot_get(&root, c"values".as_ptr());
        assert_eq!(dexbot_type(&root), 5);
        assert_eq!(dexbot_type(values), 4);
        assert_eq!(dexbot_size(&root), 1);
        assert_eq!(dexbot_size(values), 4);
        for (index, expected) in [0, 1, 2, 3].into_iter().enumerate() {
            assert_eq!(dexbot_type(dexbot_at(values, index)), expected);
        }
        assert!(dexbot_is_null(dexbot_at(values, 0)));
        assert!(!dexbot_is_null(values));
        let mut boolean = false;
        assert!(dexbot_boolean(dexbot_at(values, 1), &mut boolean));
        assert!(boolean);
        let mut number = 0.0;
        assert!(dexbot_number(dexbot_at(values, 2), &mut number));
        assert_eq!(number, 12.5);
        let mut length = 0;
        let string = dexbot_string(dexbot_at(values, 3), &mut length);
        assert_eq!(
            std::slice::from_raw_parts(string.cast::<u8>(), length),
            "a\u{0}β".as_bytes()
        );

        // Both outputs own their storage independently of the source document.
        let keys = dexbot_keys(&root);
        let json = dexbot_json(&root);
        assert!(!keys.is_null());
        assert!(!json.is_null());
        drop(root);
        assert_eq!(*keys, serde_json::json!(["values"]));
        let parsed: Value = serde_json::from_slice(CStr::from_ptr(json).to_bytes()).unwrap();
        assert_eq!(parsed["values"][3], "a\u{0}β");
        dexbot_document_free(keys);
        dexbot_string_free(json);
        dexbot_string_free(ptr::null_mut());
    }
}

#[test]
fn invalid_accessors_fail_without_overwriting_numeric_outputs() {
    let wrong_type = Value::Null;
    unsafe {
        let mut number = 42.0;
        let mut boolean = true;
        assert!(!dexbot_number(&wrong_type, &mut number));
        assert_eq!(number, 42.0);
        assert!(!dexbot_boolean(&wrong_type, &mut boolean));
        assert!(boolean);
        assert!(!dexbot_boolean(&Value::Bool(true), ptr::null_mut()));
        assert!(dexbot_get(&wrong_type, ptr::null()).is_null());
        assert!(dexbot_keys(&wrong_type).is_null());
        assert_eq!(dexbot_size(&wrong_type), 0);
        assert!(dexbot_json(ptr::null()).is_null());
        let mut length = 42;
        assert!(dexbot_string(&wrong_type, &mut length).is_null());
        assert_eq!(length, 0);
        assert!(dexbot_string(&Value::String("valid".into()), ptr::null_mut()).is_null());
    }
}

#[test]
fn error_messages_are_thread_local_and_escape_nul_characters() {
    error("parent\0error");
    std::thread::spawn(|| {
        error("child error");
        unsafe {
            assert_eq!(
                CStr::from_ptr(dexbot_last_error()).to_bytes(),
                b"child error"
            );
        }
    })
    .join()
    .unwrap();
    unsafe {
        assert_eq!(
            CStr::from_ptr(dexbot_last_error()).to_bytes(),
            b"parent\\0error"
        );
    }
}

#[test]
fn owned_profile_and_urdf_results_can_be_released() {
    unsafe {
        let profiles = dexbot_profiles();
        assert!(!profiles.is_null());
        assert_eq!(dexbot_size(profiles), model::available_profiles().len());
        dexbot_document_free(profiles);
        let profile = dexbot_profile_for(c"dm/vg0123456789-1p".as_ptr());
        assert!(!profile.is_null());
        assert_eq!(*profile, Value::String("vega_1p".into()));
        dexbot_document_free(profile);
        assert!(dexbot_profile_for(c"unknown".as_ptr()).is_null());
        let urdf = dexbot_parse_urdf(c"<robot name='custom'><link name='base'/></robot>".as_ptr());
        assert!(!urdf.is_null());
        assert_eq!((&*urdf)["robot_name"], "custom");
        dexbot_document_free(urdf);
    }
}
