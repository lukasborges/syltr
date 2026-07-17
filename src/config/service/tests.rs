use super::{apply_background_defaults, Service};

#[test]
fn missing_background_setting_defaults_to_disabled() {
    let service: Service = serde_json::from_str(
        r#"{
            "id": "example",
            "name": "Example",
            "url": "https://example.com/"
        }"#,
    )
    .expect("legacy service should deserialize");

    assert!(!service.background);
}

#[test]
fn legacy_services_receive_catalog_recommendations_once() {
    let json = r#"[
        {
            "id": "whatsapp",
            "name": "WhatsApp",
            "url": "https://web.whatsapp.com/"
        },
        {
            "id": "teams",
            "name": "Teams",
            "url": "https://teams.microsoft.com/"
        },
        {
            "id": "explicit",
            "name": "Explicit",
            "url": "https://web.whatsapp.com/",
            "background": false
        }
    ]"#;
    let mut services: Vec<Service> = serde_json::from_str(json).expect("valid services");

    assert!(apply_background_defaults(json, &mut services));
    assert!(services[0].background);
    assert!(!services[1].background);
    assert!(!services[2].background);

    let saved = serde_json::to_string(&services).expect("services should serialize");
    assert!(!apply_background_defaults(&saved, &mut services));
}
