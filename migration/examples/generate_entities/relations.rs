use heck::ToUpperCamelCase;
use migration::sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use std::{error::Error, path::Path};

pub async fn ordered_relations(
    database: &DatabaseConnection,
) -> Result<Vec<(String, String, String, String)>, Box<dyn Error>> {
    let rows = database
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT source.relname AS source_table, target.relname AS target_table,
                json_agg(source_column.attname ORDER BY key.ordinality) AS source_columns,
                json_agg(target_column.attname ORDER BY key.ordinality) AS target_columns
         FROM pg_catalog.pg_constraint constraint_info
         JOIN pg_catalog.pg_class source ON source.oid=constraint_info.conrelid
         JOIN pg_catalog.pg_namespace source_schema ON source_schema.oid=source.relnamespace
         JOIN pg_catalog.pg_class target ON target.oid=constraint_info.confrelid
         JOIN pg_catalog.pg_namespace target_schema ON target_schema.oid=target.relnamespace
         CROSS JOIN LATERAL unnest(constraint_info.conkey,constraint_info.confkey)
           WITH ORDINALITY AS key(source_number,target_number,ordinality)
         JOIN pg_catalog.pg_attribute source_column
           ON source_column.attrelid=source.oid AND source_column.attnum=key.source_number
         JOIN pg_catalog.pg_attribute target_column
           ON target_column.attrelid=target.oid AND target_column.attnum=key.target_number
         WHERE constraint_info.contype='f' AND cardinality(constraint_info.conkey)>1
           AND source_schema.nspname='public' AND target_schema.nspname='public'
         GROUP BY constraint_info.oid,source.relname,target.relname
         ORDER BY source.relname,constraint_info.oid",
        ))
        .await?;
    rows.into_iter()
        .map(|row| {
            let source: String = row.try_get("", "source_table")?;
            let target: String = row.try_get("", "target_table")?;
            let from: Vec<String> = serde_json::from_value(row.try_get("", "source_columns")?)?;
            let to: Vec<String> = serde_json::from_value(row.try_get("", "target_columns")?)?;
            let columns = |columns: Vec<String>, prefix: &str| {
                format!(
                    "({})",
                    columns
                        .into_iter()
                        .map(|column| format!("{prefix}Column::{}", column.to_upper_camel_case()))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            Ok((
                source,
                target.clone(),
                columns(from, ""),
                columns(to, &format!("super::{target}::")),
            ))
        })
        .collect()
}

pub fn reconcile_relation(
    input: &str,
    target: &str,
    from: &str,
    to: &str,
) -> Result<String, Box<dyn Error>> {
    let belongs = format!("belongs_to = \"super::{target}::Entity\"");
    let source = format!("from = \"{from}\"");
    let mut matches = Vec::new();
    for (start, _) in input.match_indices("#[sea_orm(") {
        let end = start
            + input[start..]
                .find(")]")
                .ok_or("Unclosed relation attribute")?;
        let block = &input[start..end];
        if block.contains(&belongs) && block.contains(&source) {
            let marker = "\n        to = \"";
            let to_start =
                start + block.find(marker).ok_or("Missing target columns")? + marker.len();
            let to_end = to_start
                + input[to_start..end]
                    .find('"')
                    .ok_or("Unclosed target columns")?;
            matches.push((to_start, to_end));
        }
    }
    let [(start, end)] = matches.as_slice() else {
        return Err(format!(
            "Expected one {target} relation from {from}, found {}",
            matches.len()
        )
        .into());
    };
    let mut result = input.to_owned();
    result.replace_range(*start..*end, to);
    Ok(result)
}

pub async fn reconcile(database: &DatabaseConnection, output: &Path) -> Result<(), Box<dyn Error>> {
    for (source, target, from, to) in ordered_relations(database).await? {
        let path = output.join(format!("{source}.rs"));
        let input = std::fs::read_to_string(&path)?;
        let result = reconcile_relation(&input, &target, &from, &to)?;
        if input != result {
            std::fs::write(path, result)?;
        }
    }
    Ok(())
}
