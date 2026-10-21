use crate::models::{access::TenantTransaction, errors::OperationError};
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde::Serialize;
use serde_json::json;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MemberRole {
    #[default]
    All,
    Owner,
    Administrator,
    Member,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MemberStatus {
    #[default]
    All,
    Active,
    Suspended,
    Revoked,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MemberSort {
    #[default]
    Name,
    Email,
    Role,
    Status,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortDirection {
    #[default]
    Asc,
    Desc,
}

#[derive(Clone, Debug, Default)]
pub struct MemberQuery {
    pub search: String,
    pub role: MemberRole,
    pub status: MemberStatus,
    pub sort: MemberSort,
    pub direction: SortDirection,
    pub page: u64,
    pub notice: Option<String>,
}

impl MemberQuery {
    pub fn from_form(
        form: &std::collections::HashMap<String, String>,
    ) -> Result<Self, OperationError> {
        let encoded = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(
                ["q", "role", "status", "sort", "direction", "page"]
                    .into_iter()
                    .filter_map(|key| form.get(key).map(|value| (key, value.as_str()))),
            )
            .finish();
        Self::parse(Some(&encoded)).map(|(query, _)| query)
    }
    pub fn with_page(&self, page: u64) -> Self {
        let mut query = self.clone();
        query.page = page;
        query
    }
    pub fn parse(raw: Option<&str>) -> Result<(Self, Option<String>), OperationError> {
        let mut query = Self {
            page: 1,
            ..Self::default()
        };
        let mut seen = std::collections::HashSet::new();
        let mut language = None;
        for (key, value) in url::form_urlencoded::parse(raw.unwrap_or_default().as_bytes()) {
            if !seen.insert(key.to_string()) {
                return Err(invalid(&key));
            }
            match key.as_ref() {
                "q" if value.len() <= 100 => query.search = value.trim().to_owned(),
                "role" => {
                    query.role = match value.as_ref() {
                        "all" => MemberRole::All,
                        "owner" => MemberRole::Owner,
                        "administrator" => MemberRole::Administrator,
                        "member" => MemberRole::Member,
                        _ => return Err(invalid("role")),
                    }
                }
                "status" => {
                    query.status = match value.as_ref() {
                        "all" => MemberStatus::All,
                        "active" => MemberStatus::Active,
                        "suspended" => MemberStatus::Suspended,
                        "revoked" => MemberStatus::Revoked,
                        _ => return Err(invalid("status")),
                    }
                }
                "sort" => {
                    query.sort = match value.as_ref() {
                        "name" => MemberSort::Name,
                        "email" => MemberSort::Email,
                        "role" => MemberSort::Role,
                        "status" => MemberSort::Status,
                        _ => return Err(invalid("sort")),
                    }
                }
                "direction" => {
                    query.direction = match value.as_ref() {
                        "asc" => SortDirection::Asc,
                        "desc" => SortDirection::Desc,
                        _ => return Err(invalid("direction")),
                    }
                }
                "page" => {
                    query.page = value
                        .parse::<u64>()
                        .ok()
                        .filter(|page| (1..=1_000_000).contains(page))
                        .ok_or_else(|| invalid("page"))?
                }
                "lang" => language = Some(value.into_owned()),
                "notice" if matches!(value.as_ref(), "suspended" | "restored" | "revoked") => {
                    query.notice = Some(value.into_owned())
                }
                _ => return Err(invalid(&key)),
            }
        }
        Ok((query, language))
    }

    pub fn role_value(&self) -> &'static str {
        match self.role {
            MemberRole::All => "all",
            MemberRole::Owner => "owner",
            MemberRole::Administrator => "administrator",
            MemberRole::Member => "member",
        }
    }
    pub fn status_value(&self) -> &'static str {
        match self.status {
            MemberStatus::All => "all",
            MemberStatus::Active => "active",
            MemberStatus::Suspended => "suspended",
            MemberStatus::Revoked => "revoked",
        }
    }
    pub fn sort_value(&self) -> &'static str {
        match self.sort {
            MemberSort::Name => "name",
            MemberSort::Email => "email",
            MemberSort::Role => "role",
            MemberSort::Status => "status",
        }
    }
    pub fn direction_value(&self) -> &'static str {
        match self.direction {
            SortDirection::Asc => "asc",
            SortDirection::Desc => "desc",
        }
    }
    pub fn suffix(&self, language: Option<&str>) -> String {
        let mut pairs = vec![
            ("q", self.search.as_str()),
            ("role", self.role_value()),
            ("status", self.status_value()),
            ("sort", self.sort_value()),
            ("direction", self.direction_value()),
        ];
        let page = self.page.to_string();
        pairs.push(("page", &page));
        if let Some(language) = language {
            pairs.push(("lang", language));
        }
        format!(
            "?{}",
            url::form_urlencoded::Serializer::new(String::new())
                .extend_pairs(pairs)
                .finish()
        )
    }
}

fn invalid(field: &str) -> OperationError {
    OperationError::Validation {
        details: json!({"errors":{field:["is invalid"]}}),
    }
}

#[derive(Serialize)]
pub struct MemberRow {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub role: String,
    pub status: String,
}
#[derive(Serialize)]
pub struct MemberPage {
    pub members: Vec<MemberRow>,
    pub total: i64,
    pub page: u64,
    pub pages: u64,
}

pub async fn list(
    tenant: &TenantTransaction,
    query: &MemberQuery,
) -> Result<MemberPage, OperationError> {
    super::authorize(tenant).await?;
    let search = format!(
        "%{}%",
        query
            .search
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    );
    let role = if query.role == MemberRole::All {
        ""
    } else {
        query.role_value()
    };
    let status = if query.status == MemberStatus::All {
        ""
    } else {
        query.status_value()
    };
    let values = vec![
        tenant.scope().household_id.into(),
        search.into(),
        role.into(),
        status.into(),
    ];
    let base = " FROM household_memberships m LEFT JOIN accounts a ON a.person_id=m.person_id LEFT JOIN people p ON p.id=m.person_id WHERE m.household_id=$1 AND ($2='%%' OR coalesce(nullif(a.name,''),p.name,a.email,'Member #'||m.id::text) ILIKE $2 ESCAPE '\\' OR coalesce(a.email,p.email,'') ILIKE $2 ESCAPE '\\') AND ($3='' OR m.role=$3) AND ($4='' OR m.status=$4)";
    let count_sql = format!("SELECT count(*) AS total{base}");
    let count = tenant
        .transaction()
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            count_sql,
            values.clone(),
        ))
        .await?
        .ok_or(OperationError::Unavailable)?;
    let total: i64 = count.try_get("", "total")?;
    let pages = (total.max(1) as u64).div_ceil(25);
    let page = query.page.min(pages);
    let order = " ORDER BY CASE WHEN $5='name' AND $6='asc' THEN lower(coalesce(nullif(a.name,''),p.name,a.email,'Member #'||m.id::text)) END ASC, CASE WHEN $5='name' AND $6='desc' THEN lower(coalesce(nullif(a.name,''),p.name,a.email,'Member #'||m.id::text)) END DESC, CASE WHEN $5='email' AND $6='asc' THEN lower(coalesce(a.email,p.email,'')) END ASC, CASE WHEN $5='email' AND $6='desc' THEN lower(coalesce(a.email,p.email,'')) END DESC, CASE WHEN $5='role' AND $6='asc' THEN m.role END ASC, CASE WHEN $5='role' AND $6='desc' THEN m.role END DESC, CASE WHEN $5='status' AND $6='asc' THEN m.status END ASC, CASE WHEN $5='status' AND $6='desc' THEN m.status END DESC, m.id ASC LIMIT 25 OFFSET $7";
    let sql = format!(
        "SELECT m.id,coalesce(nullif(a.name,''),p.name,a.email,'Member #'||m.id::text) AS name,coalesce(a.email,p.email,'') AS email,m.role,m.status{base}{order}"
    );
    let mut page_values = values;
    page_values.extend([
        query.sort_value().into(),
        query.direction_value().into(),
        (((page - 1) * 25) as i64).into(),
    ]);
    let rows = tenant
        .transaction()
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            page_values,
        ))
        .await?;
    let members = rows
        .into_iter()
        .map(|row| {
            Ok(MemberRow {
                id: row.try_get("", "id")?,
                name: row.try_get("", "name")?,
                email: row.try_get("", "email")?,
                role: row.try_get("", "role")?,
                status: row.try_get("", "status")?,
            })
        })
        .collect::<Result<Vec<_>, sea_orm::DbErr>>()?;
    Ok(MemberPage {
        members,
        total,
        page,
        pages,
    })
}

pub async fn find(tenant: &TenantTransaction, id: i64) -> Result<MemberRow, OperationError> {
    super::authorize(tenant).await?;
    let sql = "SELECT m.id,coalesce(nullif(a.name,''),p.name,a.email,'Member #'||m.id::text) AS name,coalesce(a.email,p.email,'') AS email,m.role,m.status FROM household_memberships m LEFT JOIN accounts a ON a.person_id=m.person_id LEFT JOIN people p ON p.id=m.person_id LEFT JOIN users u ON u.person_id=m.person_id WHERE m.household_id=$1 AND (m.id=$2 OR u.id=$2) ORDER BY CASE WHEN m.id=$2 THEN 0 ELSE 1 END,m.id LIMIT 1";
    let row = tenant
        .transaction()
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            [tenant.scope().household_id.into(), id.into()],
        ))
        .await?
        .ok_or(OperationError::NotFound)?;
    Ok(MemberRow {
        id: row.try_get("", "id")?,
        name: row.try_get("", "name")?,
        email: row.try_get("", "email")?,
        role: row.try_get("", "role")?,
        status: row.try_get("", "status")?,
    })
}
