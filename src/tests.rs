mod tests {
    #[test]
    fn test_traitement_requete_ogn() {
        use chrono::prelude::*;
        use db_interaction_server::requete_ogn;

        let date = NaiveDate::from_ymd_opt(2023, 04, 21).unwrap();
        assert_eq!(requete_ogn(date), "{\"a_day\":\"Fri\",\"airfield\":{\"code\":\"LFLE\",\"country\":\"FR\",\"elevation\":297,\"latlng\":[45.56055,5.97584],\"name\":\"Chambery Challes les Eaux\",\"time_info\":{\"dawn\":\"06h09\",\"noon\":\"13h35\",\"sunrise\":\"06h41\",\"sunset\":\"20h30\",\"twilight\":\"21h01\",\"tz_name\":\"Europe/Paris\",\"tz_offset\":\"CEST+0200\"}},\"call_tsp\":1682244524,\"code\":\"LFLE\",\"date\":\"2023-04-21\",\"devices\":[{\"address\":\"3849F2\",\"aircraft\":\"DR-300\",\"aircraft_type\":2,\"competition\":null,\"db_org\":\"OGN\",\"device_type\":\"F\",\"identified\":true,\"registration\":\"F-BSPS\",\"tracked\":true}],\"flights\":[{\"device\":0,\"duration\":2208,\"max_alt\":1318,\"max_height\":1021,\"start\":\"16h32\",\"start_q\":32,\"start_tsp\":1682087575,\"stop\":\"17h09\",\"stop_q\":32,\"stop_tsp\":1682089783,\"tow\":null,\"towing\":false,\"warn\":false}],\"rnames\":[\"LFLE\"]}\n".to_string())
    }
}