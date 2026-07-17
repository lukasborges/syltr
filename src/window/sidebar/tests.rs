use super::{current_group_index, should_keep_view};
use crate::config::Service;

fn service(background: bool, disabled: bool) -> Service {
    Service {
        id: "example".to_string(),
        name: "Example".to_string(),
        url: "https://example.com/".to_string(),
        muted: false,
        disabled,
        background,
        user_agent: None,
    }
}

#[test]
fn selected_service_is_kept_without_background_activity() {
    assert!(should_keep_view(&service(false, false), Some("example")));
}

#[test]
fn unselected_service_requires_background_activity() {
    assert!(!should_keep_view(&service(false, false), Some("another")));
    assert!(should_keep_view(&service(true, false), Some("another")));
}

#[test]
fn disabled_service_is_never_kept() {
    assert!(!should_keep_view(&service(true, true), Some("example")));
}

#[test]
fn no_service_is_selected_without_a_current_id() {
    let groups = vec![vec![service(false, false)]];

    assert_eq!(current_group_index(&groups, None), None);
    assert_eq!(current_group_index(&groups, Some("missing")), None);
    assert_eq!(current_group_index(&groups, Some("example")), Some(0));
}
