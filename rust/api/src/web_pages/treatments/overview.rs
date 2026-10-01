use super::*;
use medtracker_web::household_i18n::Text;
use medtracker_web::treatments::TreatmentRow;

pub(in crate::web_pages) async fn rows(
    api: &mut WebApi,
    household: i64,
    person: i64,
) -> Result<Vec<TreatmentRow>, PageError> {
    let medications = api
        .collection(&format!("/api/v1/households/{household}/medications"))
        .await?;
    let text = Text::new(api.locale);
    let mut rows = Vec::new();
    for kind in [Kind::Assignment, Kind::Schedule] {
        let sources = api
            .collection(&format!(
                "/api/v1/households/{household}/{}",
                kind.resource()
            ))
            .await?;
        for source in sources
            .into_iter()
            .filter(|source| numeric(source, "person_id") == Some(person))
        {
            let id = numeric(&source, "id").ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
            let medication = medications
                .iter()
                .find(|medication| numeric(medication, "id") == numeric(&source, "medication_id"))
                .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
            let key = match kind {
                Kind::Assignment => format!(
                    "person_medications.form.administration_kinds.{}.label",
                    field(&source, "administration_kind")
                ),
                Kind::Schedule => format!("treatments.types.{}", field(&source, "schedule_type")),
            };
            let description = text
                .get(&key, &[])
                .map_err(|_| error(StatusCode::BAD_GATEWAY))?;
            let paused = source
                .get("paused")
                .and_then(Value::as_bool)
                .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
            rows.push(TreatmentRow {
                id: id.to_string(),
                resource: kind.browser_resource().into(),
                medication_name: field(medication, "name").into(),
                dose_amount: field(&source, "dose_amount").into(),
                dose_unit: field(&source, "dose_unit").into(),
                notes: field(&source, "notes").into(),
                description,
                paused,
            });
        }
    }
    Ok(rows)
}
