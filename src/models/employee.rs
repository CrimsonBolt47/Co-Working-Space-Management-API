use serde::{Deserialize, Serialize};

use sqlx::{Type};
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

#[derive(Serialize, Deserialize)]
pub struct Employee{
   pub emp_id: Uuid,
   pub name: String,
   pub position: String,
   pub comp_id: Uuid,
   pub email: String,
   pub password_hash: String,
   pub role: Role,
   pub created_at: OffsetDateTime,
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