use crate::notifier::flap_guard::AlertBatch;
use serde_json::{Value, json};

pub fn build_slack_payload(batch: &AlertBatch) -> Value {
    let alert = &batch.last_alert;
    let summary = if batch.count > 1 {
        format!(
            "{} similar alerts were grouped by Flap Guard over the last window.",
            batch.count
        )
    } else {
        "Immediate delivery. No Flap Guard grouping was applied.".to_string()
    };
    let mut fields = vec![
        json!({ "type": "mrkdwn", "text": format!("*Severity*\n{}", alert.severity.as_str()) }),
        json!({ "type": "mrkdwn", "text": format!("*Device*\n{} ({})", alert.device_name, alert.device_ip) }),
        json!({ "type": "mrkdwn", "text": format!("*Alert kind*\n{}", alert.alert_kind) }),
        json!({ "type": "mrkdwn", "text": format!("*Occurred at*\n{}", alert.occurred_at) }),
    ];

    if let Some(interface_name) = &alert.interface_name {
        fields
            .push(json!({ "type": "mrkdwn", "text": format!("*Interface*\n{}", interface_name) }));
    }
    if let Some(observed_value) = &alert.observed_value {
        fields.push(json!({ "type": "mrkdwn", "text": format!("*Observed*\n{}", observed_value) }));
    }
    if let Some(threshold) = &alert.threshold {
        fields.push(json!({ "type": "mrkdwn", "text": format!("*Threshold*\n{}", threshold) }));
    }

    let action_text = alert
        .actions_or_defaults()
        .into_iter()
        .map(|action| format!("• {action}"))
        .collect::<Vec<_>>()
        .join("\n");

    json!({
        "text": format!("[{}] {}", alert.severity.as_str(), alert.title),
        "blocks": [
            {
                "type": "header",
                "text": {
                    "type": "plain_text",
                    "text": format!("{} - {}", alert.severity.as_str(), alert.title),
                    "emoji": false
                }
            },
            {
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": alert.message
                }
            },
            {
                "type": "section",
                "fields": fields
            },
            {
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": format!("*Recommended actions*\n{}", action_text)
                }
            },
            {
                "type": "context",
                "elements": [
                    {
                        "type": "mrkdwn",
                        "text": summary
                    }
                ]
            }
        ]
    })
}
