use axum::{
    Router,
    routing::{delete, get, patch, post},
};
use sqlx::PgPool;

use crate::route::{admin::login_admin, booking::{cancel_booking, create_booking, extend_booking, get_booking_by_id, get_company_bookings, get_own_bookings}, company::{create_company, delete_company, get_companies, get_company_by_id, get_my_company, update_companies}, employee::{create_employee, delete_employees, email_verification, get_employee_by_id, get_employees, login_employee, update_employees}, spaces::{create_spaces, delete_space, get_available_spaces_at_given_time, get_booked_time_spaces_by_id, get_spaces, get_spaces_by_id, update_space}};

pub fn create_router(pool: PgPool) -> Router {
    Router::new()
    //admin
    .route("/auth/admin/login",post(login_admin))        //login for admin
    //companies
    .route("/companies", post(create_company))           //create company by admin only
    .route("/companies/{id}",get(get_company_by_id))     //get company by id admin only
    .route("/companies",get(get_companies))              //list all companies by filter admin only
    .route("/companies/{id}",patch(update_companies))    //update company admin only
    .route("/companies/{id}",delete(delete_company))     //delete company (cascade delete employees) admin only
    .route("/me/company",get(get_my_company))            //for employees only
    //employees
    .route("/employees/{id}/verify",patch(email_verification))  //email verification for employees and manager to create their own password(cant do anything until they activate thier account)
    .route("/auth/login/employee",post(login_employee))         //login employees
    .route("/employees",post(create_employee))                  //add employee by manager of their own company only
    .route("/employees/{id}",get(get_employee_by_id))           //get employee by id by managers of their own company only
    .route("/employees",get(get_employees))                     //list of employees by managers only of their own company
    .route("/employees/{id}",patch(update_employees))           //update employee details by managers of their own company
    .route("/employees/{id}",delete(delete_employees))          //delete company by managers of their own company
    //spaces
    .route("/spaces",post(create_spaces))                                 //add a space by admin only
    .route("/spaces/{id}",get(get_spaces_by_id))                          //get space by id 
    .route("/spaces",get(get_spaces))                                     //list all spaces
    .route("/spaces/{id}",patch(update_space))                            //update space by admin only
    .route("/spaces/{id}",delete(delete_space))                           //delete space by admin only
    .route("/spaces/available", get(get_available_spaces_at_given_time))  //get available spaces based on givem time range
    .route("/spaces/{id}/bookings", get(get_booked_time_spaces_by_id))    //get booked times given space_id
    //booking
    .route("/bookings", post(create_booking))                         //make a booking of a space max 2 hours at a time
    .route("/bookings/{id}",delete(cancel_booking))                   //cancel booking by booking id
    .route("/bookings/{id}",patch(extend_booking))                    //extend booking time
    .route("/bookings/company",get(get_company_bookings))             //get your companies all booking by managers only
    .route("/bookings/me",get(get_own_bookings))                      //list booking history by you
    .route("/bookings/{id}",get(get_booking_by_id))                   //get booking by id of yours
    .with_state(pool)
}