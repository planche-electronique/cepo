#[cfg(test)]
#[test]
fn test_flight_from_json() {
    use brick_ogn::flight::Flight;
    use chrono::NaiveTime;

    let flight = Flight {
        ogn_nb: 1,
        takeoff_code: String::from(""),
        takeoff_machine: String::from(""),
        takeoff_machine_pilot: String::from(""),
        glider: String::from("F-CEAF"),
        flight_code: String::from(""),
        pilot1: String::from(""),
        pilot2: String::from(""),
        takeoff: NaiveTime::from_hms_opt(14, 14, 0).unwrap(),
        landing: NaiveTime::from_hms_opt(14, 19, 0).unwrap(),
    };

    let flight_json = serde_json::to_string(&flight).unwrap_or_default();
    let flight_test = serde_json::from_str(&flight_json).unwrap_or_default();
    assert_eq!(flight, flight_test)
}
