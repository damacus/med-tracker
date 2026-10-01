use crate::household::{household_document, path_segment};
use crate::household_i18n::{Locale, Text, TranslationError};
use leptos::prelude::*;

#[derive(Clone, Debug, Default)]
pub struct PersonDraft {
    pub name: String,
    pub email: String,
    pub date_of_birth: String,
    pub person_type: String,
    pub has_capacity: String,
}

#[derive(Clone, Debug)]
pub struct PersonRow {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub date_of_birth: String,
    pub person_type: String,
    pub has_capacity: bool,
    pub can_edit: bool,
}

pub fn render_people(
    household_name: &str,
    slug: &str,
    _csrf: &str,
    locale: Locale,
    people: Vec<PersonRow>,
    can_create: bool,
) -> Result<String, TranslationError> {
    let raw_slug = slug;
    let slug = path_segment(slug);
    let text = Text::new(locale);
    let title = text.get("layouts.sidebar.people", &[])?;
    let add = text.get("people.form.new_heading", &[])?;
    let body = view! {
        <div class="household-heading"><h1>{title.clone()}</h1>
            {can_create.then(|| view! { <a class="med-button" href=format!("/households/{slug}/people/new")>{add}</a> })}
        </div>
        <div class="household-grid">
            {people.into_iter().map(|person| view! {
                <article class="med-card"><h2><a href=format!("/households/{slug}/people/{}", person.id)>{person.name}</a></h2><p>{person.date_of_birth}</p></article>
            }).collect_view()}
        </div>
    }.to_html();
    Ok(household_document(
        &title,
        household_name,
        raw_slug,
        locale.as_str(),
        body,
    ))
}

pub fn render_person(
    household_name: &str,
    slug: &str,
    _csrf: &str,
    locale: Locale,
    person: PersonRow,
) -> Result<String, TranslationError> {
    let raw_slug = slug;
    let slug = path_segment(slug);
    let text = Text::new(locale);
    let name = text.get("people.form.name", &[])?;
    let email = text.get("people.form.email", &[])?;
    let dob = text.get("people.form.date_of_birth", &[])?;
    let kind = text.get("people.form.person_type", &[])?;
    let capacity = text.get("people.overview.capacity", &[])?;
    let capacity_value = text.get(
        if person.has_capacity {
            "people.overview.has_capacity"
        } else {
            "people.overview.dependent"
        },
        &[],
    )?;
    let person_type = text.get(
        &format!(
            "forms.medications.wizard.dose.person_types.{}",
            person.person_type
        ),
        &[],
    )?;
    let edit = text.get("people.show.edit_person", &[])?;
    let back = text.get("people.show.back", &[])?;
    let title = person.name.clone();
    let body = view! {
        <div class="household-heading"><h1>{person.name.clone()}</h1>
            {person.can_edit.then(|| view! { <a class="med-button" href=format!("/households/{slug}/people/{}/edit", person.id)>{edit}</a> })}
        </div>
        <section class="med-card"><dl class="household-details">
            <dt>{name}</dt><dd>{person.name}</dd><dt>{email}</dt><dd>{person.email}</dd>
            <dt>{dob}</dt><dd>{person.date_of_birth}</dd><dt>{kind}</dt><dd>{person_type}</dd>
            <dt>{capacity}</dt><dd>{capacity_value}</dd>
        </dl></section>
        <a class="med-text-button" href=format!("/households/{slug}/people")>{back}</a>
    }.to_html();
    Ok(household_document(
        &title,
        household_name,
        raw_slug,
        locale.as_str(),
        body,
    ))
}

pub fn render_person_form(
    household_name: &str,
    slug: &str,
    csrf: &str,
    locale: Locale,
    person_id: Option<i64>,
    draft: PersonDraft,
    errors: Vec<(String, String)>,
) -> Result<String, TranslationError> {
    let raw_slug = slug;
    let slug = path_segment(slug);
    let text = Text::new(locale);
    let errors = errors
        .into_iter()
        .map(|(field, message)| text.form_error(&message).map(|message| (field, message)))
        .collect::<Result<Vec<_>, TranslationError>>()?;
    let title = text.get(
        if person_id.is_some() {
            "people.form.edit_heading"
        } else {
            "people.form.new_heading"
        },
        &[],
    )?;
    let submit = text.get(
        if person_id.is_some() {
            "people.form.update"
        } else {
            "people.form.create"
        },
        &[],
    )?;
    let cancel = text.get("people.form.cancel", &[])?;
    let name = text.get("people.form.name", &[])?;
    let email = text.get("people.form.email", &[])?;
    let dob = text.get("people.form.date_of_birth", &[])?;
    let kind = text.get("people.form.person_type", &[])?;
    let capacity = text.get("people.form.has_capacity", &[])?;
    let hint = text.get("people.form.capacity_hint", &[])?;
    let choices = ["adult", "minor", "dependent_adult"]
        .into_iter()
        .map(|value| {
            Ok((
                value,
                text.get(
                    &format!("forms.medications.wizard.dose.person_types.{value}"),
                    &[],
                )?,
            ))
        })
        .collect::<Result<Vec<_>, TranslationError>>()?;
    let action = person_id.map_or_else(
        || format!("/households/{slug}/people"),
        |id| format!("/households/{slug}/people/{id}"),
    );
    let cancel_href = person_id.map_or_else(
        || format!("/households/{slug}/people"),
        |id| format!("/households/{slug}/people/{id}"),
    );
    let error_for = |field: &str| {
        errors
            .iter()
            .filter(|(key, _)| key == field)
            .map(|(_, message)| message.clone())
            .collect::<Vec<_>>()
            .join("; ")
    };
    let name_error = error_for("name");
    let email_error = error_for("email");
    let dob_error = error_for("date_of_birth");
    let kind_error = error_for("person_type");
    let capacity_error = error_for("has_capacity");
    let unknown_kind = !["adult", "minor", "dependent_adult"].contains(&draft.person_type.as_str());
    let body = view! {
        <h1>{title.clone()}</h1>
        <form class="household-form med-card" id="person_form" method="post" action=action>
            <input type="hidden" name="authenticity_token" value=csrf.to_owned()/>
            {(!errors.is_empty()).then(|| view! { <div class="med-alert" role="alert"><ul>{errors.into_iter().map(|(_, message)| view! { <li>{message}</li> }).collect_view()}</ul></div> })}
            <div class="form-field"><label for="person_name">{name}</label>
                <input id="person_name" name="name" type="text" value=draft.name required aria-invalid=(!name_error.is_empty()).then_some("true") aria-describedby=(!name_error.is_empty()).then_some("person_name_error")/>
                {(!name_error.is_empty()).then(|| view! { <p class="household-field-error" id="person_name_error">{name_error}</p> })}
            </div>
            <div class="form-field"><label for="person_email">{email}</label>
                <input id="person_email" name="email" type="email" value=draft.email aria-invalid=(!email_error.is_empty()).then_some("true") aria-describedby=(!email_error.is_empty()).then_some("person_email_error")/>
                {(!email_error.is_empty()).then(|| view! { <p class="household-field-error" id="person_email_error">{email_error}</p> })}
            </div>
            <div class="form-field"><label for="person_date_of_birth">{dob}</label>
                <input id="person_date_of_birth" name="date_of_birth" type="date" value=draft.date_of_birth required aria-invalid=(!dob_error.is_empty()).then_some("true") aria-describedby=(!dob_error.is_empty()).then_some("person_date_of_birth_error")/>
                {(!dob_error.is_empty()).then(|| view! { <p class="household-field-error" id="person_date_of_birth_error">{dob_error}</p> })}
            </div>
            <div class="form-field"><label for="person_person_type">{kind}</label>
                <select id="person_person_type" name="person_type" required aria-invalid=(!kind_error.is_empty()).then_some("true") aria-describedby=(!kind_error.is_empty()).then_some("person_person_type_error")>
                    {unknown_kind.then(|| view! { <option value=draft.person_type.clone() selected>{draft.person_type.clone()}</option> })}
                    {choices.into_iter().map(|(value,label)| view! { <option value=value selected=draft.person_type == value>{label}</option> }).collect_view()}
                </select>
                {(!kind_error.is_empty()).then(|| view! { <p class="household-field-error" id="person_person_type_error">{kind_error}</p> })}
            </div>
            <div class="form-field"><label for="person_has_capacity">{capacity}</label>
                <input id="person_has_capacity" name="has_capacity" type="checkbox" value="true" checked=draft.has_capacity == "true" aria-invalid=(!capacity_error.is_empty()).then_some("true") aria-describedby=if capacity_error.is_empty() { "person_capacity_hint" } else { "person_capacity_hint person_has_capacity_error" }/>
                <p id="person_capacity_hint">{hint}</p>
                {(!capacity_error.is_empty()).then(|| view! { <p class="household-field-error" id="person_has_capacity_error">{capacity_error}</p> })}
            </div>
            <div class="household-actions"><button class="med-button" type="submit">{submit}</button><a class="med-text-button" href=cancel_href>{cancel}</a></div>
        </form>
    }.to_html();
    Ok(household_document(
        &title,
        household_name,
        raw_slug,
        locale.as_str(),
        body,
    ))
}
