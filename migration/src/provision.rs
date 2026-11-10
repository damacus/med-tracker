use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{DatabaseConnection, TransactionTrait};

#[derive(Clone, Copy)]
pub enum Access {
    Runtime,
    Update,
}

impl Access {
    pub fn group(self) -> &'static str {
        match self {
            Self::Runtime => "med_tracker_app",
            Self::Update => "med_tracker_owner",
        }
    }
}

pub async fn lock<C: ConnectionTrait>(db: &C) -> Result<(), DbErr> {
    let query = Query::select()
        .expr(Func::cust(Alias::new("pg_advisory_xact_lock")).arg(5567948073909809714_i64))
        .to_owned();
    db.query_one_raw(db.get_database_backend().build(&query))
        .await?;
    Ok(())
}

pub async fn provision(
    db: &DatabaseConnection,
    username: &str,
    password: &str,
    access: Access,
) -> Result<(), DbErr> {
    let transaction = db.begin().await?;
    lock(&transaction).await?;
    let query = Query::select()
        .column(Alias::new("rolname"))
        .from((Alias::new("pg_catalog"), Alias::new("pg_roles")))
        .and_where(Expr::col(Alias::new("rolname")).eq(username))
        .to_owned();
    if transaction
        .query_one_raw(transaction.get_database_backend().build(&query))
        .await?
        .is_none()
    {
        // SeaQuery and SchemaManager have no CREATE ROLE or role-membership DDL.
        // PostgreSQL format quotes identifiers and passwords; values are bound
        // through SeaORM rather than interpolated into SQL or logged by setup.
        let query = Query::select()
            .expr_as(
                Func::cust(Alias::new("format")).args([
                    Expr::val("CREATE ROLE %I LOGIN INHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD %L IN ROLE %I"),
                    Expr::val(username),
                    Expr::val(password),
                    Expr::val(access.group()),
                ]),
                Alias::new("ddl"),
            )
            .to_owned();
        let ddl = transaction
            .query_one_raw(transaction.get_database_backend().build(&query))
            .await?
            .ok_or_else(|| DbErr::Custom("Role statement unavailable".into()))?
            .try_get::<String>("", "ddl")?;
        transaction.execute_unprepared(&ddl).await?;
    }
    validate(&transaction, username, access).await?;
    transaction.commit().await
}

pub async fn preflight<C: ConnectionTrait>(
    db: &C,
    username: &str,
    access: Access,
) -> Result<(), DbErr> {
    let query = Query::select()
        .column(Alias::new("rolname"))
        .from((Alias::new("pg_catalog"), Alias::new("pg_roles")))
        .and_where(Expr::col(Alias::new("rolname")).eq(username))
        .to_owned();
    if db
        .query_one_raw(db.get_database_backend().build(&query))
        .await?
        .is_some()
    {
        validate(db, username, access).await?;
    }
    Ok(())
}

pub async fn validate<C: ConnectionTrait>(
    db: &C,
    username: &str,
    access: Access,
) -> Result<(), DbErr> {
    let query = Query::select()
        .columns([
            Alias::new("rolsuper"),
            Alias::new("rolcreatedb"),
            Alias::new("rolcreaterole"),
            Alias::new("rolreplication"),
            Alias::new("rolbypassrls"),
            Alias::new("rolcanlogin"),
            Alias::new("rolinherit"),
        ])
        .expr_as(
            Func::cust(Alias::new("pg_has_role")).args([
                Expr::col(Alias::new("oid")),
                Expr::val(access.group()),
                Expr::val("MEMBER"),
            ]),
            Alias::new("member"),
        )
        .from((Alias::new("pg_catalog"), Alias::new("pg_roles")))
        .and_where(Expr::col(Alias::new("rolname")).eq(username))
        .to_owned();
    let role = db
        .query_one_raw(db.get_database_backend().build(&query))
        .await?
        .ok_or_else(|| DbErr::Custom("Database login is absent".into()))?;
    for flag in [
        "rolsuper",
        "rolcreatedb",
        "rolcreaterole",
        "rolreplication",
        "rolbypassrls",
    ] {
        if role.try_get::<bool>("", flag)? {
            return Err(DbErr::Custom(
                "Database login has incompatible privileges".into(),
            ));
        }
    }
    for flag in ["rolcanlogin", "rolinherit", "member"] {
        if !role.try_get::<bool>("", flag)? {
            return Err(DbErr::Custom(
                "Database login has incompatible membership".into(),
            ));
        }
    }
    let query = Query::select()
        .column(Alias::new("rolname"))
        .from((Alias::new("pg_catalog"), Alias::new("pg_roles")))
        .and_where(Expr::col(Alias::new("rolname")).is_not_in([username, access.group()]))
        .and_where(
            Expr::expr(Func::cust(Alias::new("pg_has_role")).args([
                Expr::val(username),
                Expr::col(Alias::new("oid")),
                Expr::val("MEMBER"),
            ]))
            .eq(true),
        )
        .to_owned();
    if db
        .query_one_raw(db.get_database_backend().build(&query))
        .await?
        .is_some()
    {
        return Err(DbErr::Custom(
            "Database login has unexpected inherited access".into(),
        ));
    }
    Ok(())
}
