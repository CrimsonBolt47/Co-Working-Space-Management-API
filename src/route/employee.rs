use axum::{Json, extract::{State, Path, Query}, http::StatusCode};
use axum_extra::{TypedHeader, headers::{Authorization, authorization::Bearer}};
use chrono::{Utc, Duration};
use bcrypt::{verify, hash};
use jsonwebtoken::{EncodingKey, Header,encode};
use serde_json::{json,Value};
use sqlx::{PgPool, QueryBuilder};
use uuid::Uuid;
use crate::{models::{admin::{Admin, AuthAdmin, LoginAdmin}, company::{Company, CreateCompanyReq}, employee::{Employee, EmployeeInvite, EmployeePassword, EmployeeQueryParams, GetEmployee, LoginEmployee, Role, UpdateEmployeeReq}}, utils::{errorhandler::AppError, jwt::{AccessRole, Claims, verify_auth_token}}};

pub async fn login_employee(
    State(pg): State<PgPool>,
    Json(payload): Json<LoginEmployee>
) -> Result<Json<Value>, AppError> {

    if payload.email.trim().is_empty() {
        return Err(AppError::validation("email is required"))
    }
    if payload.password.trim().is_empty() {
        return Err(AppError::validation("wrong credentials"))
    }

    let employee_opt = sqlx::query_as!(
            Employee,
            r#"
            SELECT 
                emp_id, name, position, comp_id, email, password_hash, created_at,
                role as "role!: Role" 
            FROM employees 
            WHERE email = $1
            "#,
            payload.email
        )
        .fetch_optional(&pg)
        .await
        .map_err(AppError::from)?;

    let employee = match employee_opt {
        Some(e) => e,
        None => return Err(AppError::not_found("employee not found"))
    };
    let employee_password_hash = match employee.password_hash {
        Some(pass) => pass,
        None => return Err(AppError::unauthorized("activate your credentials"))
    };

    let employee_role = match employee.role {
        Role::EMP => AccessRole::Employee,
        Role::MNG => AccessRole::Manager
    };

    let valid = verify(&payload.password,&employee_password_hash)
        .map_err(|_| AppError::unauthorized("invalid credentials"))?;

    if !valid {
        return Err(AppError::unauthorized("invalid credentials"));
    }
    
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "mysecret".into());
    let exp = Utc::now() + Duration::hours(1);

    let claims = Claims{
        id: employee.emp_id,
        sub: employee.email,
        role: employee_role,
        exp: exp.timestamp() as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes())
    ).map_err(|_| AppError::Unexpected)?;

    Ok(Json(json!({"token":token})))
}


pub async fn email_verification(
    State(pg): State<PgPool>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<EmployeePassword>
) -> Result<(StatusCode,Json<Value>), AppError> {

    if payload.password.trim().is_empty() {
        return Err(AppError::bad_request("password is required"))
    }
    let mut tx = pg.begin().await.map_err(AppError::from)?;
    let claims = verify_auth_token(TypedHeader(auth)).await.map_err(|_| AppError::unauthorized("do not have access"))?;
    let employee_row = sqlx::query!("SELECT emp_id FROM employees WHERE email = $1 and password_hash is NULL", claims.sub)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| AppError::unauthorized("do not have access"))?;

    let hashed = hash(payload.password, 12)
        .map_err(|_| AppError::BadRequest("bad request".into()))?;

    sqlx::query!(
        r#"
            Update employees 
            set password_hash = $1
            where emp_id = $2
            and password_hash is NULL
        "#,
        hashed,
        employee_row.emp_id
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::from)?;

    tx.commit().await.map_err(AppError::from)?;
    Ok((StatusCode::ACCEPTED, Json(json!({"success": true}))))
}

pub async fn create_employee(
    State(pg): State<PgPool>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<EmployeeInvite>
) -> Result<(StatusCode,Json<Value>), AppError> {

        //check if its manager or not
        let claims = verify_auth_token(TypedHeader(auth))
            .await
            .map_err(|_| AppError::unauthorized("do not have access for token"))?;

        if claims.role != AccessRole::Manager {
            return Err(AppError::forbidden("only managers have access"));
        }

        //get the comp_id
        let mut tx = pg.begin().await.map_err(AppError::from)?;

        let manager_row = sqlx::query!("select comp_id from employees where emp_id = $1",claims.id)
        .fetch_one(&pg)
        .await
        .map_err(AppError::from)?;


        //add employee
        let employee_row = sqlx::query!("insert into employees (name, email,comp_id, position,role) values ($1,$2,$3,$4,$5::employee_role) returning emp_id",
            payload.name,
            payload.email,
            manager_row.comp_id,
            payload.position,
            Role::EMP as Role,
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(AppError::from)?;
        
        //create token
        let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "mysecret".into());
            let exp = Utc::now() + Duration::hours(1);
        
            let claims = Claims{
                id: employee_row.emp_id,
                sub: payload.email.clone(),
                role: AccessRole::Employee,
                exp: exp.timestamp() as usize,
            };

            let token = encode(
                &Header::default(),
                &claims,
                &EncodingKey::from_secret(secret.as_bytes()),
            ).map_err(|_| AppError::Unexpected)?;
        tx.commit().await.map_err(AppError::from)?;
    Ok((StatusCode::CREATED, Json(json!({"new_password token": token}))))
}

pub async fn get_employee_by_id(
    State(pg): State<PgPool>,
    Path(emp_id): Path<Uuid>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Value>, AppError> {

    let claims = verify_auth_token(TypedHeader(auth)).await.map_err(|_| AppError::unauthorized("do not have access"))?;
    if claims.role != AccessRole::Manager {
        return Err(AppError::forbidden("only managers have access"));
    }

    let manager_company_id = sqlx::query!("select comp_id from employees where emp_id = $1",claims.id)
            .fetch_one(&pg)
            .await
            .map_err(AppError::from)?;

    let employee = sqlx::query_as!(GetEmployee, r#"select 
                emp_id,name, position, email,role as "role!: Role"
                from employees 
                where emp_id = $1 and comp_id=$2"#, emp_id, manager_company_id.comp_id)
        .fetch_one(&pg)
        .await
        .map_err(|e| 
            if let sqlx::Error::RowNotFound = e {
            AppError::not_found("Cant find the employee")
        } else {
            AppError::from(e)
        })?;

    Ok(Json(json!(employee)))
}


pub async fn get_employees(
    State(pg): State<PgPool>,
    Query(params): Query<EmployeeQueryParams>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Value>, AppError> {
    
    let claims = verify_auth_token(TypedHeader(auth)).await.map_err(|_| AppError::unauthorized("do not have access"))?;
    if claims.role != AccessRole::Manager {
        return Err(AppError::forbidden("only Managers have access"));
    }

    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(10);
    let offset = (page-1) * limit;

    let manager_company_id = sqlx::query!("select comp_id from employees where emp_id = $1",claims.id)
        .fetch_one(&pg)
        .await
        .map_err(AppError::from)?;

    let mut query_builder = QueryBuilder::new("SELECT * from employees WHERE comp_id = ");
    query_builder.push_bind(manager_company_id.comp_id);


    //name filter
    if let Some(name) = params.name{
        query_builder.push(" AND name ILIKE ");
        query_builder.push_bind(format!("%{}%", name));
    };
    if let Some(position) = params.position{
        query_builder.push(" AND position ILIKE ");
        query_builder.push_bind(format!("%{}%", position));
    };


    query_builder.push(" ORDER BY id DESC ");
    query_builder.push(" LIMIT ");
    query_builder.push_bind(limit);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    let query = query_builder.build_query_as::<GetEmployee>();

    let employees = query.fetch_all(&pg)
        .await
    .map_err(|_| AppError::not_found("not found"))?;

    let total_count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM employees")
        .fetch_one(&pg)
        .await
    .map_err(|_| AppError::not_found("not found"))?
    .unwrap_or(0);

    //response
    let response = serde_json::json!({
        "page": page,
        "limit": limit,
        "total": total_count,
        "data": employees
    });
    Ok(Json(response))
}


pub async fn update_employees(
    State(pg): State<PgPool>,
    Path(emp_id): Path<Uuid>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<UpdateEmployeeReq>
) -> Result<Json<Value>, AppError> 
{
    
    let claims = verify_auth_token(TypedHeader(auth)).await.map_err(|_| AppError::unauthorized("do not have access"))?;
    if claims.role != AccessRole::Manager {
        return Err(AppError::forbidden("only Manager have access"));
    }

    let manager_company_id = sqlx::query!("select comp_id from employees where emp_id = $1",claims.id)
        .fetch_one(&pg)
        .await
        .map_err(AppError::from)?;

    let mut query_builder = sqlx::QueryBuilder::new("UPDATE employees");
    let mut separated = query_builder.separated("SET ");
    let mut has_update = false;

    let mut tx = pg.begin().await.map_err(AppError::from)?;

    if let Some(name) = payload.name {
        separated.push("name = ");
        separated.push_bind(name);
        has_update = true;
    }

    if let Some(about) = payload.position {
        separated.push("position = ");
        separated.push_bind(about);
        has_update = true;
    }

    if !has_update {
        return Err(AppError::bad_request("no parameters provided"));
    }

    separated.push("where emp_id = ");
    separated.push_bind(emp_id);

    separated.push("and comp_id = ");
    separated.push_bind(manager_company_id.comp_id);

    let query = query_builder.build_query_as::<GetEmployee>();

    let employees = query.fetch_one(&mut *tx)
        .await
    .map_err(|_| AppError::not_found("not found"))?;

    tx.commit().await?;

    Ok(Json(json!(employees)))
}

pub async fn delete_employees(
    State(pg): State<PgPool>,
    Path(emp_id): Path<Uuid>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<StatusCode,AppError> {

    let claims = verify_auth_token(TypedHeader(auth)).await.map_err(|_| AppError::unauthorized("do not have access"))?;
    if claims.role != AccessRole::Admin {
        return Err(AppError::forbidden("only administrators have access"));
    }

    let manager_company_id = sqlx::query!("select comp_id from employees where emp_id = $1",claims.id)
    .fetch_one(&pg)
    .await
    .map_err(AppError::from)?;

    sqlx::query!("delete from employees where emp_id = $1 and comp_id = $2",emp_id, manager_company_id.comp_id)
    .execute(&pg)
    .await
    .map_err(AppError::from)?;

    Ok(StatusCode::OK)
}







