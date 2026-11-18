use crate::notifier::flap_guard::AlertBatch;
use serde_json::{Value, json};

pub fn build_teams_payload(batch: &AlertBatch) -> Value {
    let alert = &batch.last_alert;
    let action_facts = alert
        .actions_or_defaults()
        .into_iter()
        .enumerate()
        .map(|(index, action)| json!({ "title": format!("{}.", index + 1), "value": action }))
        .collect::<Vec<_>>();
    let interface = alert.interface_name.as_deref().unwrap_or("N/A");
    let observed = alert.observed_value.as_deref().unwrap_or("N/A");
    let threshold = alert.threshold.as_deref().unwrap_or("N/A");
    let flap_summary = if batch.count > 1 {
        format!(
            "{} alerts grouped, {} suppressed",
            batch.count,
            batch.suppressed_count()
        )
    } else {
        "Immediate delivery".to_string()
    };

    json!({
        "type": "message",
        "attachments": [
            {
                "contentType": "application/vnd.microsoft.card.adaptive",
                "contentUrl": null,
                "content": {
                    "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
                    "type": "AdaptiveCard",
                    "version": "1.4",
                    "body": [
                        {
                            "type": "Container",
                            "style": "emphasis",
                            "items": [
                                {
                                    "type": "TextBlock",
                                    "text": format!("{} - {}", alert.severity.as_str(), alert.title),
                                    "weight": "Bolder",
                                    "size": "Large",
                                    "wrap": true,
                                    "color": teams_color(alert.severity)
                                },
                                {
                                    "type": "TextBlock",
                                    "text": alert.message,
                                    "wrap": true
                                }
                            ]
                        },
                        {
                            "type": "FactSet",
                            "facts": [
                                { "title": "Severity", "value": alert.severity.as_str() },
                                { "title": "Device", "value": format!("{} ({})", alert.device_name, alert.device_ip) },
                                { "title": "Interface", "value": interface },
                                { "title": "Alert kind", "value": alert.alert_kind },
                                { "title": "Observed", "value": observed },
                                { "title": "Threshold", "value": threshold },
                                { "title": "Occurred at", "value": alert.occurred_at },
                                { "title": "Flap Guard", "value": flap_summary }
                            ]
                        },
                        {
                            "type": "TextBlock",
                            "text": "Recommended actions",
                            "weight": "Bolder",
                            "spacing": "Medium"
                        },
                        {
                            "type": "FactSet",
                            "facts": action_facts
                        }
                    ],
                    "msteams": {
                        "width": "Full"
                    }
                }
            }
        ]
    })
}

fn teams_color(severity: crate::notifier::NotificationSeverity) -> &'static str {
    match severity {
        crate::notifier::NotificationSeverity::Info => "Accent",
        crate::notifier::NotificationSeverity::Warning => "Warning",
        crate::notifier::NotificationSeverity::Critical => "Attention",
        crate::notifier::NotificationSeverity::Offline => "Attention",
    }
}
