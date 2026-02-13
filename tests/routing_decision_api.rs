use payments_gateway::domain::routing_decision::RoutingDecisionRecord;

#[test]
fn routing_decision_record_serializes() {
    let rec = RoutingDecisionRecord {
        payment_id: uuid::Uuid::new_v4(),
        selected_gateway: "hdfc_mock".to_string(),
        selected_score: 0.91,
        runner_up_gateway: Some("icici_mock".to_string()),
        runner_up_score: Some(0.82),
        strategy: "SCORING_ENGINE".to_string(),
        reason_summary: "top score selected".to_string(),
        score_breakdown_json: serde_json::json!({"success_rate_score": 0.95}),
        ranked_gateways_json: serde_json::json!([{"gateway_id":"hdfc_mock","score":0.91}]),
        created_at: chrono::Utc::now(),
    };

    let s = serde_json::to_string(&rec).unwrap();
    assert!(s.contains("selected_gateway"));
}
