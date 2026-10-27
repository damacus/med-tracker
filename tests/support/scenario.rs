use med_tracker::models::{
    _entities::{accounts, household_memberships, households, identity_onboarding, people, users},
    access::{Actor, HouseholdScope},
};
use sea_orm::sea_query::{Expr, ExprTrait, Func, Query};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait, Database, DatabaseConnection};

pub struct Scenario {
    pub admin: DatabaseConnection,
    pub runtime: DatabaseConnection,
    pub account: accounts::Model,
    pub household: households::Model,
    pub membership: household_memberships::Model,
}

pub async fn setup_connection() -> DatabaseConnection {
    assert_eq!(
        std::env::var("MEDTRACKER_OWNED_DATABASE").as_deref(),
        Ok("1")
    );
    let run = uuid::Uuid::parse_str(&std::env::var("MEDTRACKER_TEST_RUN_ID").unwrap()).unwrap();
    let uri = url::Url::parse(
        &std::env::var("MEDTRACKER_TEST_SETUP_DATABASE_URL")
            .expect("nextest must prepare the shared API database"),
    )
    .unwrap();
    let database = format!("api_run_{}", run.simple());
    assert_eq!(uri.path(), format!("/{database}"));
    let connection = Database::connect(uri.as_str()).await.unwrap();
    let query = Query::select()
        .expr_as(
            Func::cust("shobj_description").args([Expr::col("oid"), Expr::val("pg_database")]),
            "owner",
        )
        .from(("pg_catalog", "pg_database"))
        .and_where(Expr::col("datname").eq(&database))
        .to_owned();
    let marker = connection
        .query_one_raw(connection.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap()
        .try_get::<Option<String>>("", "owner")
        .unwrap();
    assert_eq!(marker, Some(format!("medtracker-api-run:{run}")));
    connection
}

impl Scenario {
    pub async fn new() -> Self {
        let admin = setup_connection().await;
        let runtime = Database::connect(std::env::var("DATABASE_URL").unwrap())
            .await
            .unwrap();
        let unique = uuid::Uuid::new_v4().simple().to_string();
        let email = format!("carer-{unique}@example.test");
        let now = chrono::Utc::now();
        let person = people::ActiveModel {
            name: Set("Synthetic adult".into()),
            date_of_birth: Set(chrono::NaiveDate::from_ymd_opt(1985, 2, 3)),
            person_type: Set(0),
            has_capacity: Set(true),
            created_at: Set(now.naive_utc()),
            updated_at: Set(now.naive_utc()),
            ..Default::default()
        }
        .insert(&admin)
        .await
        .unwrap();
        let account = accounts::ActiveModel {
            person_id: Set(person.id),
            email: Set(email.clone()),
            status: Set(2),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&admin)
        .await
        .unwrap();
        let verified = Query::update()
            .table("accounts")
            .value("email_verified", true)
            .and_where(Expr::col("id").eq(account.id))
            .to_owned();
        admin
            .execute_raw(admin.get_database_backend().build(&verified))
            .await
            .unwrap();
        #[derive(serde::Deserialize)]
        struct CredentialFixture {
            password: String,
        }
        static PASSWORD: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
            serde_yaml_ng::from_str::<Vec<CredentialFixture>>(include_str!(
                "../../src/fixtures/auth_credentials.yaml"
            ))
            .unwrap()
            .into_iter()
            .next()
            .expect("fixture credential")
            .password
        });
        let credential = Query::insert()
            .into_table("auth_credentials")
            .columns([
                "id",
                "account_id",
                "provider_id",
                "user_id",
                "password",
                "created_at",
                "updated_at",
            ])
            .values([
                Expr::val(uuid::Uuid::new_v4().to_string()),
                Expr::val(account.id.to_string()),
                Expr::val("credential"),
                Expr::val(account.id),
                Expr::val(PASSWORD.clone()),
                Expr::val(now),
                Expr::val(now),
            ])
            .unwrap()
            .to_owned();
        admin
            .execute_raw(admin.get_database_backend().build(&credential))
            .await
            .unwrap();
        identity_onboarding::ActiveModel {
            account_id: Set(account.id),
            completed_at: Set(Some(now.fixed_offset())),
        }
        .insert(&admin)
        .await
        .unwrap();
        users::ActiveModel {
            person_id: Set(person.id),
            email_address: Set(email),
            active: Set(true),
            created_at: Set(now.naive_utc()),
            updated_at: Set(now.naive_utc()),
            ..Default::default()
        }
        .insert(&admin)
        .await
        .unwrap();
        let household = households::ActiveModel {
            created_by_account_id: Set(Some(account.id)),
            name: Set("Synthetic household".into()),
            slug: Set(format!("household-{unique}")),
            timezone: Set("UTC".into()),
            created_at: Set(now.naive_utc()),
            updated_at: Set(now.naive_utc()),
            ..Default::default()
        }
        .insert(&admin)
        .await
        .unwrap();
        let membership = household_memberships::ActiveModel {
            household_id: Set(household.id),
            person_id: Set(person.id),
            role: Set("administrator".into()),
            status: Set("active".into()),
            joined_at: Set(now.naive_utc()),
            created_at: Set(now.naive_utc()),
            updated_at: Set(now.naive_utc()),
            ..Default::default()
        }
        .insert(&admin)
        .await
        .unwrap();
        Self {
            admin,
            runtime,
            account,
            household,
            membership,
        }
    }

    pub fn scope(&self) -> HouseholdScope {
        HouseholdScope {
            actor: Actor {
                account_id: self.account.id,
            },
            household_id: self.household.id,
            request_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    pub async fn database_name(&self) -> String {
        let query = Query::select()
            .expr_as(Func::cust("current_database"), "database_name")
            .to_owned();
        self.runtime
            .query_one_raw(self.runtime.get_database_backend().build(&query))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "database_name")
            .unwrap()
    }

    pub async fn close(self) {
        self.runtime.close().await.unwrap();
        self.admin.close().await.unwrap();
    }
}
