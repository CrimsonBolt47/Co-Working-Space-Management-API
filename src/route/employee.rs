use axum::{Json, extract::{State, Path, Query}, http::StatusCode};
use axum_extra::{TypedHeader, headers::{Authorization, authorization::Bearer}};
use bcrypt::{verify, hash};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde_json::{json, Value};
use sqlx::PgPool;
use tracing::warn;
use uuid::Uuid;
use crate::{models::{employee::{Employee, EmployeeInvite, EmployeePassword, EmployeeQueryParams, GetEmployee, LoginEmployee, Role, UpdateEmployeeReq}}, utils::{errorhandler::AppError, jwt::{AccessRole, Claims, verify_auth_token}}};

pub async fn login_employee(
    State(pg): State<PgPool>,
    Json(payload): Json<LoginEmployee>
) -> Result<Json<Value>, AppError> {

    if payload.email.trim().is_empty() {
        return Err(AppError::bad_request("invalid credentials"))
    }
    if payload.password.trim().is_empty() {
        return Err(AppError::bad_request("invalid credentials"))
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
        .map_err(|e| {
            warn!("Database error fetching employee: {}", e);
            AppError::database("Failed to fetch employee")
        })?;

    let employee = match employee_opt {
        Some(e) => e,
        None => {
            warn!("Failed login attempt: employee not found for email: {}", payload.email);
            return Err(AppError::unauthorized("invalid credentials"))
        }
    };
    let employee_password_hash = match employee.password_hash {
        Some(pass) => pass,
        None => {
            warn!("Failed login attempt: employee has not set password for email: {}", payload.email);
            return Err(AppError::unauthorized("activate your credentials"))
        }
    };

    let employee_role = match employee.role {
        Role::EMP => AccessRole::Employee,
        Role::MNG => AccessRole::Manager
    };

    let valid = verify(&payload.password,&employee_password_hash)
        .map_err(|_| AppError::unauthorized("invalid credentials"))?;

    if !valid {
        warn!("Failed login attempt: invalid password for email: {}", payload.email);
        return Err(AppError::unauthorized("invalid credentials"));
    }
    
    let secret = std::env::var("JWT_SECRET")
        .expect("JWT_SECRET environment variable must be set");
    let token_expiry_hours: u64 = std::env::var("TOKEN_EXPIRY_HOURS")
        .ok()
        .and_then(|h| h.parse().ok())
        .unwrap_or(1);
    
    let exp = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() + (token_expiry_hours * 3600)) as usize;

    let claims = Claims{
        id: employee.emp_id,
        sub: employee.email,
        role: employee_role,
        exp,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes())
    ).map_err(|e| {
        warn!("JWT encoding failed: {}", e);
        AppError::Unexpected
    })?;

    Ok(Json(json!({
        "success": true,
        "data": {
            "token": token
        }
    })))
}


pub async fn email_verification(
    State(pg): State<PgPool>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<EmployeePassword>
) -> Result<Json<Value>, AppError> {

    if payload.password.trim().is_empty() {
        return Err(AppError::bad_request("password is required"))
    }
    
    let claims = verify_auth_token(TypedHeader(auth)).await.map_err(|_| {
        warn!("Unauthorized password verification attempt - invalid token");
        AppError::unauthorized("invalid credentials")
    })?;
    
    let mut tx = pg.begin().await.map_err(|e| {
        warn!("Database error starting transaction: {}", e);
        AppError::database("Failed to verify email")
    })?;
    
    let employee_row = sqlx::query!("SELECT emp_id FROM employees WHERE email = $1 and password_hash is NULL", claims.sub)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            match e {
                sqlx::Error::RowNotFound => {
                    warn!("Password already set for email: {}", claims.sub);
                    AppError::bad_request("account already activated")
                },
                _ => {
                    warn!("Database error fetching employee: {}", e);
                    AppError::database("Failed to verify email")
                }
            }
        })?;

    let hashed = hash(payload.password, 12)
        .map_err(|e| {
            warn!("Password hashing failed: {}", e);
            AppError::bad_request("invalid password")
        })?;

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
    .map_err(|e| {
        warn!("Database error updating password: {}", e);
        AppError::database("Failed to verify email")
    })?;

    tx.commit().await.map_err(|e| {
        warn!("Database error committing transaction: {}", e);
        AppError::database("Failed to verify email")
    })?;
    
    Ok(Json(json!({
        "success": true,
        "data": {
            "message": "Email verified successfully"
        }
    })))
}

pub async fn create_employee(
    State(pg): State<PgPool>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<EmployeeInvite>
) -> Result<(StatusCode,Json<Value>), AppError> {

        //check if its manager or not
        let claims = verify_auth_token(TypedHeader(auth))
            .await
            .map_err(|_| {
                warn!("Unauthorized employee creation attempt - invalid token");
                AppError::unauthorized("invalid credentials")
            })?;

        if claims.role != AccessRole::Manager {
            warn!("Non-manager user attempted to create employee");
            return Err(AppError::forbidden("only managers have access"));
        }

        // Validate email format
        if !payload.email.contains('@') || payload.email.trim().is_empty() {
            return Err(AppError::bad_request("invalid email format"));
        }

        //get the comp_id
        let mut tx = pg.begin().await.map_err(|e| {
            warn!("Database error starting transaction: {}", e);
            AppError::database("Failed to create employee")
        })?;

        let manager_row = sqlx::query!("select comp_id from employees where emp_id = $1", claims.id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| {
                warn!("Database error fetching manager company: {}", e);
                AppError::database("Failed to create employee")
            })?;

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
            .map_err(|e| {
                warn!("Database error inserting employee: {}", e);
                AppError::database("Failed to create employee")
            })?;
        
        //create token
        let secret = std::env::var("JWT_SECRET")
            .expect("JWT_SECRET environment variable must be set");
        let token_expiry_hours: u64 = std::env::var("TOKEN_EXPIRY_HOURS")
            .ok()
            .and_then(|h| h.parse().ok())
            .unwrap_or(1);
        let exp = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() + (token_expiry_hours * 3600)) as usize;
        
            let claims = Claims{
                id: employee_row.emp_id,
                sub: payload.email.clone(),
                role: AccessRole::Employee,
                exp,
            };

            let token = encode(
                &Header::default(),
                &claims,
                &EncodingKey::from_secret(secret.as_bytes()),
            ).map_err(|e| {
                warn!("JWT encoding failed: {}", e);
                AppError::Unexpected
            })?;
        tx.commit().await.map_err(|e| {
            warn!("Database error committing transaction: {}", e);
            AppError::database("Failed to create employee")
        })?;
    Ok((StatusCode::CREATED, Json(json!({
        "success": true,
        "data": {
            "token": token
        }
    }))))
}

pub async fn get_employee_by_id(
    State(pg): State<PgPool>,
    Path(emp_id): Path<Uuid>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Value>, AppError> {

    let claims = verify_auth_token(TypedHeader(auth)).await.map_err(|_| {
        warn!("Unauthorized get employee attempt - invalid token");
        AppError::unauthorized("invalid credentials")
    })?;
    if claims.role != AccessRole::Manager {
        warn!("Non-manager user attempted to get employee by id");
        return Err(AppError::forbidden("only managers have access"));
    }

    let manager_company_id = sqlx::query!("select comp_id from employees where emp_id = $1", claims.id)
            .fetch_one(&pg)
            .await
            .map_err(|e| {
                warn!("Database error fetching manager company: {}", e);
                AppError::database("Failed to fetch employee")
            })?;

    let employee = sqlx::query_as!(GetEmployee, r#"select 
                emp_id,name, position, email,role as "role!: Role"
                from employees 
                where emp_id = $1 and comp_id=$2"#, emp_id, manager_company_id.comp_id)
        .fetch_one(&pg)
        .await
        .map_err(|e| {
            match e {
                sqlx::Error::RowNotFound => {
                    warn!("Employee not found: {} for manager: {}", emp_id, claims.id);
                    AppError::not_found("employee not found")
                },
                _ => {
                    warn!("Database error fetching employee: {}", e);
                    AppError::database("Failed to fetch employee")
                }
            }
        })?;

    Ok(Json(json!({
        "success": true,
        "data": employee
    })))
}


pub async fn get_employees(
    State(pg): State<PgPool>,
    Query(params): Query<EmployeeQueryParams>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Value>, AppError> {
    
    let claims = verify_auth_token(TypedHeader(auth)).await.map_err(|_| {
        warn!("Unauthorized get employees attempt - invalid token");
        AppError::unauthorized("invalid credentials")
    })?;
    if claims.role != AccessRole::Manager {
        warn!("Non-manager user attempted to list employees");
        return Err(AppError::forbidden("only managers have access"));
    }

    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(10);
    let offset = (page-1) * limit;

    let manager_company_id = sqlx::query!("select comp_id from employees where emp_id = $1", claims.id)
        .fetch_one(&pg)
        .await
        .map_err(|e| {
            warn!("Database error fetching manager company: {}", e);
            AppError::database("Failed to fetch employees")
        })?;

    let mut query_builder = sqlx::QueryBuilder::new("SELECT * from employees WHERE comp_id = ");
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

    query_builder.push(" ORDER BY emp_id DESC ");
    query_builder.push(" LIMIT ");
    query_builder.push_bind(limit);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    let query = query_builder.build_query_as::<GetEmployee>();

    let employees = query.fetch_all(&pg)
        .await
        .map_err(|e| {
            warn!("Database error fetching employees: {}", e);
            AppError::database("Failed to fetch employees")
        })?;

    let total_count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM employees WHERE comp_id = $1", manager_company_id.comp_id)
        .fetch_one(&pg)
        .await
        .map_err(|e| {
            warn!("Database error counting employees: {}", e);
            AppError::database("Failed to count employees")
        })?
        .unwrap_or(0);

    //response
    let response = serde_json::json!({
        "success": true,
        "data": {
            "page": page,
            "limit": limit,
            "total": total_count,
            "items": employees
        }
    });
    Ok(Json(response))
}


pub async fn update_employees(
    State(pg): State<PgPool>,
    Path(emp_id): Path<Uuid>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<UpdateEmployeeReq>
) -> Result<Json<Value>, AppError> {
    
    let claims = verify_auth_token(TypedHeader(auth)).await.map_err(|_| {
        warn!("Unauthorized update employee attempt - invalid token");
        AppError::unauthorized("invalid credentials")
    })?;
    if claims.role != AccessRole::Manager {
        warn!("Non-manager user attempted to update employee");
        return Err(AppError::forbidden("only managers have access"));
    }

    let manager_company_id = sqlx::query!("select comp_id from employees where emp_id = $1", claims.id)
        .fetch_one(&pg)
        .await
        .map_err(|e| {
            warn!("Database error fetching manager company: {}", e);
            AppError::database("Failed to update employee")
        })?;

    let mut query_builder = sqlx::QueryBuilder::new("UPDATE employees");
    let mut separated = query_builder.separated("SET ");
    let mut has_update = false;

    if let Some(name) = payload.name {
        separated.push("name = ");
        separated.push_bind(name);
        has_update = true;
    }

    if let Some(position) = payload.position {
        separated.push("position = ");
        separated.push_bind(position);
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

    let mut tx = pg.begin().await.map_err(|e| {
        warn!("Database error starting transaction: {}", e);
        AppError::database("Failed to update employee")
    })?;

    let employee = query.fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            match e {
                sqlx::Error::RowNotFound => {
                    warn!("Employee not found for update: {} by manager: {}", emp_id, claims.id);
                    AppError::not_found("employee not found")
                },
                _ => {
                    warn!("Database error updating employee: {}", e);
                    AppError::database("Failed to update employee")
                }
            }
        })?;

    tx.commit().await.map_err(|e| {
        warn!("Database error committing transaction: {}", e);
        AppError::database("Failed to update employee")
    })?;

    Ok(Json(json!({
        "success": true,
        "data": employee
    })))
}

pub async fn delete_employees(
    State(pg): State<PgPool>,
    Path(emp_id): Path<Uuid>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<StatusCode, AppError> {

    let claims = verify_auth_token(TypedHeader(auth)).await.map_err(|_| {
        warn!("Unauthorized delete employee attempt - invalid token");
        AppError::unauthorized("invalid credentials")
    })?;
    if claims.role != AccessRole::Manager {
        warn!("Non-manager user attempted to delete employee: {}", emp_id);
        return Err(AppError::forbidden("only managers have access"));
    }

    let manager_company_id = sqlx::query!("select comp_id from employees where emp_id = $1", claims.id)
        .fetch_one(&pg)
        .await
        .map_err(|e| {
            warn!("Database error fetching manager company: {}", e);
            AppError::database("Failed to delete employee")
        })?;

    let result = sqlx::query!("delete from employees where emp_id = $1 and comp_id = $2", emp_id, manager_company_id.comp_id)
        .execute(&pg)
        .await
        .map_err(|e| {
            warn!("Database error deleting employee: {}", e);
            AppError::database("Failed to delete employee")
        })?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("employee not found"));
    }

    Ok(StatusCode::NO_CONTENT)
}







