use crate::web::routes::ApiSummary;

pub fn summary() -> ApiSummary {
    ApiSummary {
        healthy: 0,
        warning: 0,
        critical: 0,
        offline: 0,
    }
}
