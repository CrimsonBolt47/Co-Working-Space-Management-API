use serde::{Deserialize, Serialize};

use sqlx::{Type, prelude::FromRow};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Type, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[sqlx(type_name = "employee_role")] 
pub enum Role {
    #[sqlx(rename = "EMP")] 
    EMP,
    #[sqlx(rename = "MNG")]
    MNG, 
}

#[derive(Serialize, Deserialize, FromRow)]
pub struct Employee{
   pub emp_id: Uuid,
   pub name: String,
   pub position: String,
   pub comp_id: Uuid,
   pub email: String,
   pub password_hash: Option<String>,
   pub role: Role,
   pub created_at: OffsetDateTime,
}

#[derive(Serialize, Deserialize, FromRow)]
pub struct GetEmployee{
   pub emp_id: Uuid,
   pub name: String,
   pub position: String,
   pub email: String,
   pub role: Role,
}

#[derive(Deserialize)]
pub struct EmployeeInvite {
   pub name: String,
   pub position: String,
   pub email: String,

}

#[derive(Deserialize)]
pub struct EmployeePassword {
    pub password: String
}

#[derive(Deserialize)]
pub struct LoginEmployee {
    pub email: String,
    pub password: String
}

#[derive(Deserialize)]
pub struct EmployeeQueryParams {
     pub page: Option<i64>,
    pub limit: Option<i64>,
    pub name: Option<String>,
    pub position: Option<String>
}

#[derive(Deserialize)]
pub struct UpdateEmployeeReq {
    pub name: Option<String>,
    pub position: Option<String>
}