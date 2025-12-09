use axum::{
    Router,
    routing::{delete, get, patch, post},
};
use sqlx::PgPool;

use crate::route::{admin::login_admin, company::{create_company, delete_company, get_companies, get_company_by_id, get_my_company, update_companies}, employee::{create_employee, delete_employees, email_verification, get_employee_by_id, get_employees, login_employee, update_employees}};

pub fn create_router(pool: PgPool) -> Router {
    Router::new()
    //admin
    .route("/auth/admin/login",post(login_admin))
    //companies
    .route("/companies", post(create_company))
    .route("/companies:id",get(get_company_by_id))
    .route("/companies",get(get_companies))
    .route("/companies:id",patch(update_companies))
    .route("/companies:id",delete(delete_company))
    .route("/companies/my",get(get_my_company))  //for employees only
    //employees
    .route("/employees/setpassword",patch(email_verification))
    .route("/auth/login/employee",post(login_employee))
    .route("/employees",post(create_employee))
    .route("/employees:id",get(get_employee_by_id))
    .route("/employees",get(get_employees))
    .route("/employees",patch(update_employees))
    .route("/employees",delete(delete_employees))
    //spaces
    .with_state(pool)
}