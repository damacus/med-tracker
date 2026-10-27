pub fn require_owned_database() {
    assert_eq!(std::env::var("MEDTRACKER_OWNED_DATABASE").unwrap(), "1");
}
