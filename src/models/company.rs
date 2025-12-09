use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::models::employee::EmployeeInvite;



#[derive(Serialize, Deserialize,Debug, sqlx::FromRow)]
pub struct Company {
    pub comp_id: Uuid,
    pub company_name: String,
    pub about: Option<String>,
    pub created_at: OffsetDateTime,
}

#[derive(Deserialize)]
pub struct CreateCompanyReq {
    pub company_name: String,
    pub about: String,
    pub manager: EmployeeInvite
}

#[derive(Deserialize)]
pub struct CompanyQueryParams {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub company_name: Option<String>
}
