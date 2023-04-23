mod tests {
    use chrono::prelude::*;
    #[test]
    fn test_traitement_requete_ogn() {
        let date = NaiveDate::from_ymd(2023, 04, 23);
        assert_eq!(requete_ogn(date), "{\"a_day\":\"Sun\",\"airfield\":{\"code\":\"LFLE\",\"country\":\"FR\",\"elevation\":297,\"latlng\":[45.56055,5.97584],\"name\":\"Chambery Challes les Eaux\",\"time_info\":{\"dawn\":\"06h06\",\"noon\":\"13h34\",\"sunrise\":\"06h38\",\"sunset\":\"20h32\",\"twilight\":\"21h04\",\"tz_name\":\"Europe/Paris\",\"tz_offset\":\"CEST+0200\"}},\"call_tsp\":1682243909,\"code\":\"LFLE\",\"date\":\"2023-04-23\",\"devices\":[{\"address\":\"393C0F\",\"aircraft\":\"DR-400\",\"aircraft_type\":2,\"competition\":null,\"db_org\":\"OGN\",\"device_type\":\"F\",\"identified\":true,\"registration\":\"F-GPAP\",\"tracked\":true},{\"address\":\"3900F5\",\"aircraft\":\"DR-400\",\"aircraft_type\":2,\"competition\":null,\"db_org\":\"OGN\",\"device_type\":\"F\",\"identified\":true,\"registration\":\"F-GAHV\",\"tracked\":true},{\"address\":\"3849F2\",\"aircraft\":\"DR-300\",\"aircraft_type\":2,\"competition\":null,\"db_org\":\"OGN\",\"device_type\":\"F\",\"identified\":true,\"registration\":\"F-BSPS\",\"tracked\":true}],\"flights\":[{\"device\":0,\"duration\":7955,\"max_alt\":1697,\"max_height\":1400,\"start\":\"09h03\",\"start_q\":32,\"start_tsp\":1682233385,\"stop\":\"11h15\",\"stop_q\":14,\"stop_tsp\":1682241340,\"tow\":null,\"towing\":false,\"warn\":false},{\"device\":1,\"duration\":2892,\"max_alt\":1531,\"max_height\":1234,\"start\":\"10h26\",\"start_q\":15,\"start_tsp\":1682238361,\"stop\":\"11h14\",\"stop_q\":14,\"stop_tsp\":1682241253,\"tow\":null,\"towing\":false,\"warn\":false},{\"device\":2,\"duration\":2915,\"max_alt\":1574,\"max_height\":1277,\"start\":\"10h32\",\"start_q\":14,\"start_tsp\":1682238747,\"stop\":\"11h21\",\"stop_q\":14,\"stop_tsp\":1682241662,\"tow\":null,\"towing\":false,\"warn\":false}],\"rnames\":[\"LFLE\"]}".to_string())
    }
}