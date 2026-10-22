use super::{Application, require, withdrawal::declaration};
use crate::platform::{form_page, hidden_field, post_form, sign_in};
use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

#[tokio::test]
async fn guardian_recovery_browser_discloses_eligible_minor_and_requires_explicit_confirmation()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        app.fixture.admin.execute_unprepared(
            "UPDATE public.household_memberships SET role='owner' WHERE id=74001;
             INSERT INTO public.people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at)
             VALUES(73994,'Synthetic recovery child','2020-01-01',1,false,now(),now());
             INSERT INTO public.household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at)
             VALUES(74994,72001,73994,'member','active',now(),now(),now())"
        ).await?;
        let cookie=sign_in(&app,71001,"persistence@example.test").await;
        let (_,list)=form_page(&app,&cookie,"/households/persistence-fixture/admin/carer_relationships").await;
        require(list.contains("/admin/carer_relationships/recover"),"Admin relationships must link to recovery")?;
        let (_,page)=form_page(&app,&cookie,"/households/persistence-fixture/admin/carer_relationships/recover").await;
        require(page.contains("Synthetic recovery child") && page.contains("/recover/73994"),
            "Recovery list must show eligible minor")?;
        let (_,detail)=form_page(&app,&cookie,"/households/persistence-fixture/admin/carer_relationships/recover/73994").await;
        require(detail.contains("Synthetic recovery child") && detail.contains("authenticity_token")
            && detail.contains("confirm_recovery") && detail.contains("snapshot_token"),
            "Recovery detail must show CSRF, snapshot and deliberate confirmation")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_browser_appoints_with_audit_and_new_guardian_can_withdraw()
-> Result<(), DbErr> {
    let mut app = Application::new().await;
    let result = async {
        app.client = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none())
            .build().map_err(|error| DbErr::Custom(error.to_string()))?;
        app.fixture.admin.execute_unprepared(
            "UPDATE public.household_memberships SET role='owner' WHERE id=74001;
             INSERT INTO public.people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at)
             VALUES(73994,'Synthetic browser orphan','2020-01-01',1,false,now(),now()),
                   (73999,'Synthetic newly appointed adult','1980-01-01',0,true,now(),now());
             INSERT INTO public.household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at)
             VALUES(74994,72001,73994,'member','active',now(),now(),now()),
                   (74999,72001,73999,'member','active',now(),now(),now());
             INSERT INTO public.accounts(id,person_id,email,status,created_at,updated_at)
             VALUES(71999,73999,'browser-new-guardian@example.test',2,now(),now());
             INSERT INTO public.users(id,person_id,email_address,active,created_at,updated_at)
             VALUES(77999,73999,'browser-new-guardian@example.test',true,now(),now())"
        ).await?;
        crate::fixtures::seed_native_password(&app.fixture.admin,71999).await;
        let cookie=sign_in(&app,71001,"persistence@example.test").await;
        let path="/households/persistence-fixture/admin/carer_relationships/recover/73994";
        let (cookies,page)=form_page(&app,&cookie,path).await;
        let csrf=hidden_field(&page,"authenticity_token");
        let snapshot=hidden_field(&page,"snapshot_token");
        let revision=hidden_field(&page,"snapshot_revision");
        let link=hidden_field(&page,"link_token");
        let form=[("authenticity_token",csrf.as_str()),("action","appoint"),("guardian_id","73999"),
            ("snapshot_token",snapshot.as_str()),("snapshot_revision",revision.as_str()),
            ("link_token",link.as_str()),("reason","Manager confirmed a new guardian for this minor.")];
        let unconfirmed=post_form(&app,&cookies,path,&form).await;
        require(unconfirmed.status().as_u16()==422,"Recovery requires explicit server-side confirmation")?;
        let validation_page=unconfirmed.text().await.map_err(|error|DbErr::Custom(error.to_string()))?;
        require(validation_page.contains("<option value=\"73999\" selected>"),
            "Appointment validation must retain the chosen guardian")?;
        let denied=post_form(&app,&cookies,path,&[("action","appoint"),("guardian_id","73999"),
            ("snapshot_token",snapshot.as_str()),("snapshot_revision",revision.as_str()),
            ("link_token",link.as_str()),("reason","Manager confirmed a new guardian for this minor."),
            ("confirm_recovery","true")]).await;
        require(denied.status().as_u16()==403,"Recovery must reject a missing CSRF token")?;
        let malformed=post_form(&app,&cookies,path,&[form[0],form[1],form[2],
            ("snapshot_token","not-a-uuid"),form[4],form[5],form[6],("confirm_recovery","true")]).await;
        require(malformed.status().as_u16()==422,"Malformed hidden snapshot must be rejected")?;
        let malformed_page=malformed.text().await.map_err(|error|DbErr::Custom(error.to_string()))?;
        require(malformed_page.contains("href=\"#recovery-form\"")
            && malformed_page.contains("id=\"recovery-form\" tabindex=\"-1\""),
            "Hidden-field error must link to a focusable fallback when no restore form exists")?;
        let unknown_action=post_form(&app,&cookies,path,&[form[0],("action","unexpected"),form[2],form[3],form[4],form[5],form[6],("confirm_recovery","true")]).await;
        require(unknown_action.status().as_u16()==422,"Unknown action must be rejected")?;
        let unknown_page=unknown_action.text().await.map_err(|error|DbErr::Custom(error.to_string()))?;
        require(unknown_page.contains("href=\"#recovery-form\"")
            && !unknown_page.contains("href=\"#restore-reason\""),
            "Untrusted action must use the safe focusable fallback")?;
        let stale=post_form(&app,&cookies,path,&[form[0],form[1],form[2],
            ("snapshot_token","11111111-1111-4111-8111-111111111111"),form[4],form[5],form[6],
            ("confirm_recovery","true")]).await;
        require(stale.status().as_u16()==409,"Opaque stale confirmation must be rejected")?;
        let rejected=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM public.guardian_declarations WHERE subject_person_id=73994) AS declarations,
             (SELECT count(*) FROM public.carer_relationships WHERE patient_id=73994) AS links,
             (SELECT count(*) FROM public.security_audit_events WHERE event_type LIKE 'guardian_recovery.%') AS audits"))
            .await?.ok_or_else(||DbErr::Custom("Missing rejected recovery state".into()))?;
        require(rejected.try_get::<i64>("","declarations")?==0
            && rejected.try_get::<i64>("","links")?==0
            && rejected.try_get::<i64>("","audits")?==0,
            "Missing confirmation, CSRF and stale snapshot must leave no authority or recovery audit")?;
        let valid=post_form(&app,&cookies,path,&[form[0],form[1],form[2],form[3],form[4],form[5],form[6],("confirm_recovery","true")]).await;
        require(valid.status().as_u16()==303,"Confirmed recovery should redirect after appointment")?;
        let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT d.id,d.revision,(SELECT count(*) FROM public.carer_relationships c WHERE c.guardian_declaration_id=d.id AND c.active) AS links,
              (SELECT count(*) FROM public.security_audit_events a WHERE a.event_type='guardian_recovery.appointed' AND a.actor_account_id=71001 AND a.metadata->>'subject_person_id'='73994') AS audits
             FROM public.guardian_declarations d WHERE d.subject_person_id=73994 AND d.guardian_person_id=73999 AND d.source='manager_appointment' AND d.declared_by_account_id=71001"))
            .await?.ok_or_else(|| DbErr::Custom("Missing manager appointment".into()))?;
        let id:i64=row.try_get("","id")?;
        let revision:i64=row.try_get("","revision")?;
        require(row.try_get::<i64>("","links")?==1 && row.try_get::<i64>("","audits")?==1,
            "Appointment must persist exact authority and attributable audit")?;
        let guardian_cookie=sign_in(&app,71999,"browser-new-guardian@example.test").await;
        let (guardian_cookies,history)=form_page(&app,&guardian_cookie,"/profile/guardian-declarations").await;
        require(history.contains("Household manager appointment") && history.contains(&format!("/{id}/withdraw")),
            "New guardian must see truthful manager provenance and own withdrawal")?;
        let history_csrf=hidden_field(&history,"authenticity_token");
        let withdrawn=post_form(&app,&guardian_cookies,&format!("/profile/guardian-declarations/{id}/withdraw"),
            &[("authenticity_token",history_csrf.as_str()),("revision",revision.to_string().as_str()),("withdrawn","true")]).await;
        let withdrawn_status=withdrawn.status().as_u16();
        let withdrawn_body=withdrawn.text().await.map_err(|error| DbErr::Custom(error.to_string()))?;
        require(withdrawn_status==303,&format!("New guardian must be able to withdraw; status={withdrawn_status}; body={withdrawn_body}"))?;
        let (_,final_history)=form_page(&app,&guardian_cookie,"/profile/guardian-declarations").await;
        require(final_history.contains("Withdrawn") && !final_history.contains(&format!("/{id}/withdraw")),
            "Terminal withdrawal must be shown without a withdrawal action")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_browser_restores_exact_inactive_link_and_preserves_generic_denial()
-> Result<(), DbErr> {
    let mut app = Application::new().await;
    let result=async {
        app.client=reqwest::Client::builder().redirect(reqwest::redirect::Policy::none())
            .build().map_err(|error|DbErr::Custom(error.to_string()))?;
        app.fixture.admin.execute_unprepared("UPDATE public.household_memberships SET role='owner' WHERE id=74001").await?;
        let token=app.token().await;
        let (subject,declaration_id)=declaration(&app,&token).await?;
        let row=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT id FROM public.carer_relationships WHERE guardian_declaration_id=$1",[declaration_id.into()]))
            .await?.ok_or_else(||DbErr::Custom("Missing initial guardian link".into()))?;
        let link_id:i64=row.try_get("","id")?;
        app.fixture.admin.execute_unprepared(&format!("UPDATE public.carer_relationships SET active=false WHERE id={link_id}")).await?;
        app.fixture.admin.execute_unprepared(
            "UPDATE public.people SET name='Linked guardian <care>' WHERE id=73001;
             INSERT INTO public.people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at)
             VALUES(73999,'Other eligible adult','1980-01-01',0,true,now(),now());
             INSERT INTO public.household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at)
             VALUES(74999,72001,73999,'member','active',now(),now(),now());
             INSERT INTO public.accounts(id,person_id,email,status,created_at,updated_at)
             VALUES(71999,73999,'restore-other@example.test',2,now(),now());
             INSERT INTO public.users(id,person_id,email_address,active,created_at,updated_at)
             VALUES(77999,73999,'restore-other@example.test',true,now(),now())"
        ).await?;
        let cookie=sign_in(&app,71001,"persistence@example.test").await;
        let path=format!("/households/persistence-fixture/admin/carer_relationships/recover/{subject}");
        let (cookies,page)=form_page(&app,&cookie,&path).await;
        require(page.contains("Restore existing guardian") && hidden_field(&page,"relationship_id")==link_id.to_string(),
            "Only the exact inactive declaration link should be offered for restoration")?;
        let restore_card=page.split("id=\"administration-form\"").next().unwrap_or("");
        require(restore_card.contains("Linked guardian &lt;care&gt;")
            && !restore_card.contains("Other eligible adult")
            && hidden_field(&page,"guardian_id")=="73001",
            "Restore card must visibly identify the exact escaped linked guardian, independent of another eligible adult")?;
        let csrf=hidden_field(&page,"authenticity_token");
        let snapshot=hidden_field(&page,"snapshot_token");
        let revision=hidden_field(&page,"snapshot_revision");
        let link_token=hidden_field(&page,"link_token");
        let declaration=declaration_id.to_string();
        let link=link_id.to_string();
        let form=[("authenticity_token",csrf.as_str()),("action","restore"),("guardian_id","73001"),
            ("declaration_id",declaration.as_str()),("relationship_id",link.as_str()),
            ("snapshot_token",snapshot.as_str()),("snapshot_revision",revision.as_str()),
            ("link_token",link_token.as_str()),("reason","Manager restored an existing guardian link."),
            ("confirm_recovery","true")];
        let missing_confirm=post_form(&app,&cookies,&path,&form[..9]).await;
        require(missing_confirm.status().as_u16()==422,"Restore confirmation must be required")?;
        let confirmation_page=missing_confirm.text().await.map_err(|error|DbErr::Custom(error.to_string()))?;
        require(confirmation_page.contains("href=\"#restore-confirm\"")
            && confirmation_page.contains("id=\"restore-confirm\""),
            "Restore confirmation error must target the restore checkbox")?;
        let invalid_reason=post_form(&app,&cookies,&path,&[form[0],form[1],form[2],form[3],form[4],form[5],form[6],form[7],("reason","short"),form[9]]).await;
        require(invalid_reason.status().as_u16()==422,"Short restore reason must be rejected")?;
        let reason_page=invalid_reason.text().await.map_err(|error|DbErr::Custom(error.to_string()))?;
        require(reason_page.contains("href=\"#restore-reason\"")
            && reason_page.contains("id=\"restore-reason\""),
            "Restore reason error must target the restore textarea")?;
        let restored=post_form(&app,&cookies,&path,&form).await;
        require(restored.status().as_u16()==303,"Exact guardian restore must redirect after commit")?;
        let row=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT c.active,(SELECT count(*) FROM public.guardian_declarations d WHERE d.subject_person_id=$1) AS declarations,
             (SELECT count(*) FROM public.security_audit_events a WHERE a.event_type='guardian_recovery.restored' AND a.actor_account_id=71001) AS audits
             FROM public.carer_relationships c WHERE c.id=$2",[subject.into(),link_id.into()]))
            .await?.ok_or_else(||DbErr::Custom("Missing restored link".into()))?;
        require(row.try_get::<bool>("","active")? && row.try_get::<i64>("","declarations")?==1
            && row.try_get::<i64>("","audits")?==1,"Restore must reuse existing declaration/link and audit manager")?;
        let generic=post_form(&app,&cookies,&format!("/households/persistence-fixture/admin/carer_relationships/{link_id}/activate"),
            &[("authenticity_token",csrf.as_str())]).await;
        require(generic.status().as_u16()==403,"Generic activation must remain forbidden")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_browser_shows_superseded_initial_history_without_withdrawal()
-> Result<(), DbErr> {
    let mut app = Application::new().await;
    let result=async {
        app.client=reqwest::Client::builder().redirect(reqwest::redirect::Policy::none())
            .build().map_err(|error|DbErr::Custom(error.to_string()))?;
        app.fixture.admin.execute_unprepared("UPDATE public.household_memberships SET role='owner' WHERE id=74001").await?;
        let token=app.token().await;
        let (subject,old_id)=declaration(&app,&token).await?;
        app.fixture.admin.execute_unprepared(&format!(
            "UPDATE public.carer_relationships SET active=false WHERE guardian_declaration_id={old_id};
             INSERT INTO public.people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at)
             VALUES(73999,'Synthetic replacement guardian','1980-01-01',0,true,now(),now());
             INSERT INTO public.household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at)
             VALUES(74999,72001,73999,'member','active',now(),now(),now());
             INSERT INTO public.accounts(id,person_id,email,status,created_at,updated_at)
             VALUES(71999,73999,'browser-replacement@example.test',2,now(),now());
             INSERT INTO public.users(id,person_id,email_address,active,created_at,updated_at)
             VALUES(77999,73999,'browser-replacement@example.test',true,now(),now())"
        )).await?;
        let cookie=sign_in(&app,71001,"persistence@example.test").await;
        let path=format!("/households/persistence-fixture/admin/carer_relationships/recover/{subject}");
        let (cookies,page)=form_page(&app,&cookie,&path).await;
        let csrf=hidden_field(&page,"authenticity_token");
        let snapshot=hidden_field(&page,"snapshot_token");
        let revision=hidden_field(&page,"snapshot_revision");
        let link=hidden_field(&page,"link_token");
        let appointed=post_form(&app,&cookies,&path,&[("authenticity_token",csrf.as_str()),
            ("action","appoint"),("guardian_id","73999"),("snapshot_token",snapshot.as_str()),
            ("snapshot_revision",revision.as_str()),("link_token",link.as_str()),
            ("reason","Manager appointed another adult after link ended."),("confirm_recovery","true")]).await;
        require(appointed.status().as_u16()==303,"Different adult should be appointable while old link is inactive")?;
        let (_,history)=form_page(&app,&cookies,"/profile/guardian-declarations").await;
        require(history.contains("Your initial guardian declaration") && history.contains("Superseded by a later appointment")
            && !history.contains(&format!("/{old_id}/withdraw")),
            "Old guardian must see truthful superseded initial history with no terminal withdrawal control")?;
        Ok(())
    }.await;
    app.close().await;
    result
}
