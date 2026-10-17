//! Company bookkeeping; recording a transaction does not execute a payment.
use super::*;

pub(super) async fn list(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    let org = match state.daemon.orgintel.get(&company).await {
        Ok(org) => org,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "accounting",
                format!("{error:#}"),
            )
        }
    };
    match org.accounting_records().await {
        Ok(records) => {
            let source = finance::payments(&state.daemon.authority, &company).await;
            let payments_status = if source.is_ok() {
                "available"
            } else {
                "unavailable"
            };
            let payments: Vec<_> = source
                .unwrap_or_default()
                .into_iter()
                .map(|p| {
                    serde_json::json!({
                        "id": p.request.idempotency_key, "purpose": p.request.purpose,
                        "work_id": p.request.work_id, "amount_minor": p.request.amount_minor,
                        "currency": p.request.currency, "state": p.state.as_str(),
                        "occurred_at": p.settled_at.unwrap_or(p.updated_at),
                    })
                })
                .collect();
            Json(serde_json::json!({"records": records, "payments": payments, "payments_status": payments_status})).into_response()
        }
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "accounting",
            error.plain_message(),
        ),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SaveInput {
    version: i64,
    record: restless_orgintel::AccountingInput,
}

pub(super) async fn save(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, id)): AxumPath<(String, Uuid)>,
    Json(input): Json<SaveInput>,
) -> Response<Body> {
    if let Err(error) = input.record.validate() {
        return api_error(StatusCode::BAD_REQUEST, "accounting", error.plain_message());
    }
    let org = match state.daemon.orgintel.get(&company).await {
        Ok(org) => org,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "accounting",
                format!("{error:#}"),
            )
        }
    };
    match org
        .save_accounting_record(id, input.version, &input.record, principal.actor_id())
        .await
    {
        Ok(record) => Json(record).into_response(),
        Err(restless_orgintel::OrgIntelError::InvalidAccounting(message)) => {
            api_error(StatusCode::CONFLICT, "accounting", message)
        }
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "accounting",
            error.plain_message(),
        ),
    }
}
