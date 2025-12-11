# Co-Space Working API Documentation

## Overview

Co-Space is a workspace management API that enables companies to manage shared office spaces and employee bookings. The system allows administrators to oversee companies and spaces, managers to handle their company employees, and employees to book available workspace. The API enforces role-based access control with three user roles: Admin, Manager, and Employee.

## API Basics

**Base URL:** `127.0.0.1:7879` (configurable via environment variables)

**Content-Type:** All requests and responses use `application/json`

### Response Format

All successful responses follow this structure:

```json
{
  "success": true,
  "data": {
    // Response payload varies by endpoint
  }
}
```

### Error Responses

Error responses return appropriate HTTP status codes with the following structure:

```json
{
  "success": false,
  "error": "error message describing the issue"
}
```

## User Roles

The system has three distinct user roles with different access levels:

### 1. Admin
- Full system control
- Can create and manage companies
- Can manage spaces (create, update, delete)
- Cannot create bookings or manage employees directly

### 2. Manager
- Company-level administrator
- Manages employees within their company
- Can view company bookings
- Can make space bookings like employees
- Created when a company is initialized

### 3. Employee
- Regular user
- Can book available spaces (max 2 hours per booking)
- Can view their own bookings
- Cannot manage other employees or spaces
- Must verify email and set password before first login

---

## Database Schema

### admins
| Column | Type | Description |
| --- | --- | --- |
| admin_id | UUID | Primary key, unique identifier |
| email | String | Login email, unique |
| password_hash | String | Bcrypt hashed password |
| created_at | OffsetDateTime | Account creation timestamp |

### companies
| Column | Type | Description |
| --- | --- | --- |
| comp_id | UUID | Primary key, unique identifier |
| company_name | String | Name of the company |
| about | String (Optional) | Company description/bio |
| created_at | OffsetDateTime | Company creation timestamp |

### employees
| Column | Type | Description |
| --- | --- | --- |
| emp_id | UUID | Primary key, unique identifier |
| name | String | Employee full name |
| position | String | Job title/position |
| comp_id | UUID | Foreign key to companies |
| email | String | Login email, unique |
| password_hash | String (Optional) | Bcrypt hashed password (null until verified) |
| role | Enum (EMP/MNG) | Employee role - EMP for regular employee, MNG for manager |
| created_at | OffsetDateTime | Account creation timestamp |

### spaces
| Column | Type | Description |
| --- | --- | --- |
| space_id | UUID | Primary key, unique identifier |
| name | String | Space name/identifier |
| size | Integer | Capacity/size of the space |
| description | String (Optional) | Space details/amenities |
| created_at | OffsetDateTime | Space creation timestamp |

### bookings
| Column | Type | Description |
| --- | --- | --- |
| booking_id | UUID | Primary key, unique identifier |
| space_id | UUID | Foreign key to spaces |
| booked_by | UUID | Foreign key to employees (who made the booking) |
| start_time | OffsetDateTime | Booking start time |
| end_time | OffsetDateTime | Booking end time |
| created_at | OffsetDateTime | Booking creation timestamp |

**Constraints:**
- Bookings cannot exceed 2 hours duration
- Bookings cannot overlap (prevents double-booking)
- Bookings must be for today's date only
- Booking times must be in the future

---

## Endpoints

### Authentication

#### 1. Admin Login
**Endpoint:** `POST /auth/admin/login`  
**Access:** Public  
**Description:** Authenticate an admin user and receive JWT token

**Request Body:**
```json
{
  "email": "admin@example.com",
  "password": "password123"
}
```

**Response (Success - 200):**
```json
{
  "success": true,
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIs..."
  }
}
```

**Possible Errors:**
- `400` - Invalid credentials (empty email/password)
- `401` - Invalid credentials (user not found or wrong password)

---

#### 2. Employee Login
**Endpoint:** `POST /auth/login/employee`  
**Access:** Public  
**Description:** Authenticate an employee/manager and receive JWT token

**Request Body:**
```json
{
  "email": "employee@example.com",
  "password": "password123"
}
```

**Response (Success - 200):**
```json
{
  "success": true,
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIs..."
  }
}
```

**Possible Errors:**
- `400` - Invalid credentials (empty email/password)
- `401` - Invalid credentials (user not found)
- `401` - "activate your credentials" (employee hasn't set password via email verification)

---

### Company Management

#### 3. Create Company
**Endpoint:** `POST /companies`  
**Access:** Admin only  
**Authentication:** Required (Bearer token)  
**Description:** Create a new company with an initial manager

**Request Body:**
```json
{
  "company_name": "Tech Solutions Inc",
  "about": "Leading software development company",
  "manager": {
    "name": "John Smith",
    "position": "General Manager",
    "email": "john.smith@techsolutions.com"
  }
}
```

**Response (Success - 201):**
```json
{
  "success": true,
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIs..."
  }
}
```

**Notes:**
- Manager is automatically created as part of company creation
- Manager receives a JWT token for first-time login
- Manager must set their password via email verification before using system features
- Returns manager token, not company data

**Possible Errors:**
- `401` - Invalid or expired token
- `403` - User is not an admin
- `400` - Invalid email format for manager

---

#### 4. Get Company by ID
**Endpoint:** `GET /companies/{id}`  
**Access:** Admin only  
**Authentication:** Required (Bearer token)  
**Path Parameters:** `id` (company UUID)

**Response (Success - 200):**
```json
{
  "success": true,
  "data": {
    "comp_id": "550e8400-e29b-41d4-a716-446655440000",
    "company_name": "Tech Solutions Inc",
    "about": "Leading software development company",
    "created_at": "2025-01-15T10:30:00Z"
  }
}
```

**Possible Errors:**
- `401` - Invalid or expired token
- `403` - User is not an admin
- `404` - Company not found

---

#### 5. List All Companies
**Endpoint:** `GET /companies`  
**Access:** Admin only  
**Authentication:** Required (Bearer token)  
**Query Parameters:**
- `page` (optional, default: 1) - Page number for pagination
- `limit` (optional, default: 10) - Items per page
- `company_name` (optional) - Filter by company name (case-insensitive, partial match)

**Response (Success - 200):**
```json
{
  "success": true,
  "data": {
    "page": 1,
    "limit": 10,
    "total": 25,
    "items": [
      {
        "comp_id": "550e8400-e29b-41d4-a716-446655440000",
        "company_name": "Tech Solutions Inc",
        "about": "Leading software development company",
        "created_at": "2025-01-15T10:30:00Z"
      }
    ]
  }
}
```

**Possible Errors:**
- `401` - Invalid or expired token
- `403` - User is not an admin

---

#### 6. Update Company
**Endpoint:** `PATCH /companies/{id}`  
**Access:** Admin only  
**Authentication:** Required (Bearer token)  
**Path Parameters:** `id` (company UUID)

**Request Body:**
```json
{
  "company_name": "Tech Solutions Inc - Updated",
  "about": "Updated company description"
}
```

**Response (Success - 200):**
```json
{
  "success": true,
  "data": {
    "comp_id": "550e8400-e29b-41d4-a716-446655440000",
    "company_name": "Tech Solutions Inc - Updated",
    "about": "Updated company description",
    "created_at": "2025-01-15T10:30:00Z"
  }
}
```

**Notes:**
- At least one field must be provided
- Both fields are optional but at least one is required

**Possible Errors:**
- `401` - Invalid or expired token
- `403` - User is not an admin
- `400` - No parameters provided
- `404` - Company not found

---

#### 7. Delete Company
**Endpoint:** `DELETE /companies/{id}`  
**Access:** Admin only  
**Authentication:** Required (Bearer token)  
**Path Parameters:** `id` (company UUID)

**Response (Success - 200):**
```json
{
  "success": true,
  "data": {
    "message": "Company deleted successfully"
  }
}
```

**Notes:**
- Cascade deletes all associated employees
- Cascade deletes all bookings made by company employees
- Cannot be undone

**Possible Errors:**
- `401` - Invalid or expired token
- `403` - User is not an admin
- `404` - Company not found

---

#### 8. Get My Company
**Endpoint:** `GET /me/company`  
**Access:** Employee and Manager only  
**Authentication:** Required (Bearer token)  
**Description:** Get the company details for the authenticated employee/manager

**Response (Success - 200):**
```json
{
  "success": true,
  "data": {
    "comp_id": "550e8400-e29b-41d4-a716-446655440000",
    "company_name": "Tech Solutions Inc",
    "about": "Leading software development company",
    "created_at": "2025-01-15T10:30:00Z"
  }
}
```

**Possible Errors:**
- `401` - Invalid or expired token
- `403` - Admin users cannot access this endpoint
- `404` - Employee not found or company not found

---

### Employee Management

#### 9. Email Verification / Set Password
**Endpoint:** `PATCH /employees/{id}/verify`  
**Access:** Public (with bearer token)  
**Path Parameters:** `id` (employee UUID)  
**Description:** Employee sets their password during account activation. Token is provided when employee is created.

**Request Body:**
```json
{
  "password": "newpassword123"
}
```

**Response (Success - 200):**
```json
{
  "success": true,
  "data": {
    "message": "Email verified successfully"
  }
}
```

**Notes:**
- Employee must do this before they can log in normally
- Uses the token provided when invited by manager
- Password cannot be empty
- After this, employee can use `/auth/login/employee` endpoint

**Possible Errors:**
- `400` - Password is empty
- `400` - Account already activated
- `401` - Invalid or expired token

---

#### 10. Create Employee
**Endpoint:** `POST /employees`  
**Access:** Manager only  
**Authentication:** Required (Bearer token)  
**Description:** Manager invites a new employee to their company

**Request Body:**
```json
{
  "name": "Jane Doe",
  "position": "Software Engineer",
  "email": "jane.doe@company.com"
}
```

**Response (Success - 201):**
```json
{
  "success": true,
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIs..."
  }
}
```

**Notes:**
- Only managers can create employees
- Employee is created in the manager's company automatically
- Employee must use returned token to verify email and set password
- Email must be unique across system
- Token is for one-time use with `/employees/{id}/verify`

**Possible Errors:**
- `401` - Invalid or expired token
- `403` - User is not a manager
- `400` - Invalid email format

---

#### 11. Get Employee by ID
**Endpoint:** `GET /employees/{id}`  
**Access:** Manager only  
**Authentication:** Required (Bearer token)  
**Path Parameters:** `id` (employee UUID)  
**Description:** Get employee details. Manager can only see employees in their company.

**Response (Success - 200):**
```json
{
  "success": true,
  "data": {
    "emp_id": "660e8400-e29b-41d4-a716-446655440001",
    "name": "Jane Doe",
    "position": "Software Engineer",
    "email": "jane.doe@company.com",
    "role": "EMP"
  }
}
```

**Notes:**
- Password hash is not returned for security
- Managers can only see employees from their own company
- Returns GetEmployee model without password_hash and comp_id

**Possible Errors:**
- `401` - Invalid or expired token
- `403` - User is not a manager
- `404` - Employee not found

---

#### 12. List Employees
**Endpoint:** `GET /employees`  
**Access:** Manager only  
**Authentication:** Required (Bearer token)  
**Query Parameters:**
- `page` (optional, default: 1) - Page number
- `limit` (optional, default: 10) - Items per page
- `name` (optional) - Filter by employee name (case-insensitive)
- `position` (optional) - Filter by position (case-insensitive)

**Response (Success - 200):**
```json
{
  "success": true,
  "data": {
    "page": 1,
    "limit": 10,
    "total": 5,
    "items": [
      {
        "emp_id": "660e8400-e29b-41d4-a716-446655440001",
        "name": "Jane Doe",
        "position": "Software Engineer",
        "email": "jane.doe@company.com",
        "role": "EMP"
      }
    ]
  }
}
```

**Notes:**
- Managers can only see employees from their company
- All filters are optional and can be combined

**Possible Errors:**
- `401` - Invalid or expired token
- `403` - User is not a manager

---

#### 13. Update Employee
**Endpoint:** `PATCH /employees/{id}`  
**Access:** Manager only  
**Authentication:** Required (Bearer token)  
**Path Parameters:** `id` (employee UUID)

**Request Body:**
```json
{
  "name": "Jane Smith",
  "position": "Senior Software Engineer"
}
```

**Response (Success - 200):**
```json
{
  "success": true,
  "data": {
    "emp_id": "660e8400-e29b-41d4-a716-446655440001",
    "name": "Jane Smith",
    "position": "Senior Software Engineer",
    "email": "jane.doe@company.com",
    "role": "EMP"
  }
}
```

**Notes:**
- Managers can only update employees in their company
- Both fields are optional
- At least one field must be provided

**Possible Errors:**
- `401` - Invalid or expired token
- `403` - User is not a manager
- `400` - No parameters provided
- `404` - Employee not found

---

#### 14. Delete Employee
**Endpoint:** `DELETE /employees/{id}`  
**Access:** Manager only  
**Authentication:** Required (Bearer token)  
**Path Parameters:** `id` (employee UUID)

**Response (Success - 200):**
```json
{
  "success": true,
  "data": {
    "message": "Employee deleted successfully"
  }
}
```

**Notes:**
- Managers can only delete employees in their company
- Cascade deletes all bookings made by the employee
- Cannot be undone

**Possible Errors:**
- `401` - Invalid or expired token
- `403` - User is not a manager
- `404` - Employee not found

---

### Space Management

#### 15. Create Space
**Endpoint:** `POST /spaces`  
**Access:** Admin only  
**Authentication:** Required (Bearer token)

**Request Body:**
```json
{
  "name": "Conference Room A",
  "size": 10,
  "description": "Large conference room with projector and whiteboard"
}
```

**Response (Success - 201):**
```json
{
  "success": true,
  "data": {
    "space_id": "770e8400-e29b-41d4-a716-446655440002"
  }
}
```

**Notes:**
- Size must be greater than 0
- Description is optional

**Possible Errors:**
- `401` - Invalid or expired token
- `403` - User is not an admin
- `400` - Space size must be greater than 0

---

#### 16. Get Space by ID
**Endpoint:** `GET /spaces/{id}`  
**Access:** Public  
**Path Parameters:** `id` (space UUID)

**Response (Success - 200):**
```json
{
  "success": true,
  "data": {
    "space_id": "770e8400-e29b-41d4-a716-446655440002",
    "name": "Conference Room A",
    "size": 10,
    "description": "Large conference room with projector and whiteboard",
    "created_at": "2025-01-15T10:30:00Z"
  }
}
```

**Possible Errors:**
- `404` - Space not found

---

#### 17. List All Spaces
**Endpoint:** `GET /spaces`  
**Access:** Public  
**Query Parameters:**
- `page` (optional, default: 1) - Page number
- `limit` (optional, default: 10) - Items per page
- `name` (optional) - Filter by space name (case-insensitive)
- `size` (optional) - Filter by exact space size

**Response (Success - 200):**
```json
{
  "success": true,
  "data": {
    "page": 1,
    "limit": 10,
    "total": 15,
    "items": [
      {
        "space_id": "770e8400-e29b-41d4-a716-446655440002",
        "name": "Conference Room A",
        "size": 10,
        "description": "Large conference room with projector and whiteboard",
        "created_at": "2025-01-15T10:30:00Z"
      }
    ]
  }
}
```

---

#### 18. Update Space
**Endpoint:** `PATCH /spaces/{id}`  
**Access:** Admin only  
**Authentication:** Required (Bearer token)  
**Path Parameters:** `id` (space UUID)

**Request Body:**
```json
{
  "name": "Conference Room A - Renovated",
  "size": 15
}
```

**Response (Success - 200):**
```json
{
  "success": true,
  "data": {
    "space_id": "770e8400-e29b-41d4-a716-446655440002",
    "name": "Conference Room A - Renovated",
    "size": 15,
    "description": "Large conference room with projector and whiteboard",
    "created_at": "2025-01-15T10:30:00Z"
  }
}
```

**Notes:**
- Both fields are optional but at least one is required
- Size must be greater than 0

**Possible Errors:**
- `401` - Invalid or expired token
- `403` - User is not an admin
- `400` - No parameters provided or size must be greater than 0
- `404` - Space not found

---

#### 19. Delete Space
**Endpoint:** `DELETE /spaces/{id}`  
**Access:** Admin only  
**Authentication:** Required (Bearer token)  
**Path Parameters:** `id` (space UUID)

**Response (Success - 200):**
```json
{
  "success": true,
  "data": {
    "message": "Space deleted successfully"
  }
}
```

**Notes:**
- Cascade deletes all bookings for this space
- Cannot be undone

**Possible Errors:**
- `401` - Invalid or expired token
- `403` - User is not an admin
- `404` - Space not found

---

#### 20. Get Available Spaces
**Endpoint:** `GET /spaces/available`  
**Access:** Public  
**Query Parameters:**
- `start_time` (required) - ISO 8601 format timestamp
- `end_time` (required) - ISO 8601 format timestamp

**Description:** Get all spaces that have no bookings during the specified time range.

**Response (Success - 200):**
```json
{
  "success": true,
  "data": [
    {
      "space_id": "770e8400-e29b-41d4-a716-446655440002",
      "name": "Conference Room A",
      "size": 10,
      "description": "Large conference room"
    }
  ]
}
```

**Possible Errors:**
- `400` - Invalid time parameters

---

#### 21. Get Booked Times for Space
**Endpoint:** `GET /spaces/{id}/bookings`  
**Access:** Public  
**Path Parameters:** `id` (space UUID)

**Description:** Get all booked time slots for a specific space.

**Response (Success - 200):**
```json
{
  "success": true,
  "data": [
    {
      "start_time": "2025-01-15T14:00:00Z",
      "end_time": "2025-01-15T16:00:00Z"
    }
  ]
}
```

**Possible Errors:**
- `404` - Space not found

---

### Booking Management

#### 22. Create Booking
**Endpoint:** `POST /bookings`  
**Access:** Employee and Manager only  
**Authentication:** Required (Bearer token)

**Request Body:**
```json
{
  "space_id": "770e8400-e29b-41d4-a716-446655440002",
  "start_time": "2025-01-15T14:00:00Z",
  "end_time": "2025-01-15T15:30:00Z"
}
```

**Response (Success - 201):**
```json
{
  "success": true,
  "data": {
    "booking_id": "880e8400-e29b-41d4-a716-446655440003"
  }
}
```

**Notes:**
- Duration must be at least 1 hour but cannot exceed 2 hours
- Booking must be for today's date only
- Booking time must be in the future (later than current time)
- Space must be available during requested time

**Business Rules:**
- Max duration: 2 hours
- Min duration: 1 hour (implicit from max 2 hour constraint)
- Date: Today only
- Time: Future times only

**Possible Errors:**
- `401` - Invalid or expired token
- `403` - Admins cannot make bookings
- `422` - Booking date must be today
- `422` - Invalid timings (end before start, exceeds 2 hours, or less than 1 hour)
- `422` - Booking time must be in the future
- `400` - Space slot already booked (conflict exists)

---

#### 23. Cancel Booking
**Endpoint:** `DELETE /bookings/{id}`  
**Access:** Employee and Manager only  
**Authentication:** Required (Bearer token)  
**Path Parameters:** `id` (booking UUID)

**Response (Success - 200):**
```json
{
  "success": true,
  "data": {
    "message": "Booking cancelled successfully"
  }
}
```

**Notes:**
- Users can only cancel their own bookings
- Cancellation is immediate

**Possible Errors:**
- `401` - Invalid or expired token
- `403` - Admins cannot cancel bookings
- `404` - Booking not found (or doesn't belong to user)

---

#### 24. Extend Booking
**Endpoint:** `PATCH /bookings/{id}`  
**Access:** Employee and Manager only  
**Authentication:** Required (Bearer token)  
**Path Parameters:** `id` (booking UUID)

**Request Body:**
```json
{
  "extra_time": {
    "secs": 1800,
    "nanos": 0
  }
}
```

**Notes:**
- `extra_time` is a duration object (seconds and nanoseconds)
- Extending with 1800 seconds = 30 minutes extension
- Combined duration (original + extension) cannot exceed 2 hours total
- Space must be available for the extension period

**Response (Success - 200):**
```json
{
  "success": true,
  "data": {
    "booking_id": "880e8400-e29b-41d4-a716-446655440003",
    "space_id": "770e8400-e29b-41d4-a716-446655440002",
    "booked_by": "660e8400-e29b-41d4-a716-446655440001",
    "start_time": "2025-01-15T14:00:00Z",
    "end_time": "2025-01-15T15:30:00Z"
  }
}
```

**Possible Errors:**
- `401` - Invalid or expired token
- `403` - Admins cannot extend bookings
- `404` - Booking not found
- `422` - Total duration would exceed 2 hours
- `400` - Extended time conflicts with another booking

---

#### 25. Get Company Bookings
**Endpoint:** `GET /bookings/company`  
**Access:** Manager only  
**Authentication:** Required (Bearer token)

**Description:** Get all bookings made by employees in the manager's company.

**Response (Success - 200):**
```json
{
  "success": true,
  "data": [
    {
      "booking_id": "880e8400-e29b-41d4-a716-446655440003",
      "space_id": "770e8400-e29b-41d4-a716-446655440002",
      "emp_id": "660e8400-e29b-41d4-a716-446655440001",
      "employee_name": "Jane Doe",
      "email": "jane.doe@company.com",
      "start_time": "2025-01-15T14:00:00Z",
      "end_time": "2025-01-15T16:00:00Z"
    }
  ]
}
```

**Notes:**
- Only managers can see their company's bookings
- Returns employee name and email for reference
- Sorted with most recent first

**Possible Errors:**
- `401` - Invalid or expired token
- `403` - User is not a manager

---

#### 26. Get My Bookings
**Endpoint:** `GET /bookings/me`  
**Access:** Employee and Manager only  
**Authentication:** Required (Bearer token)

**Description:** Get all bookings made by the authenticated user.

**Response (Success - 200):**
```json
{
  "success": true,
  "data": [
    {
      "space_id": "770e8400-e29b-41d4-a716-446655440002",
      "booked_by": "660e8400-e29b-41d4-a716-446655440001",
      "start_time": "2025-01-15T14:00:00Z",
      "end_time": "2025-01-15T16:00:00Z"
    }
  ]
}
```

**Possible Errors:**
- `401` - Invalid or expired token
- `403` - Admins cannot view bookings

---

#### 27. Get Booking by ID
**Endpoint:** `GET /bookings/{id}`  
**Access:** Employee and Manager only  
**Authentication:** Required (Bearer token)  
**Path Parameters:** `id` (booking UUID)

**Description:** Get details of a specific booking. Users can only view their own bookings.

**Response (Success - 200):**
```json
{
  "success": true,
  "data": {
    "space_id": "770e8400-e29b-41d4-a716-446655440002",
    "booked_by": "660e8400-e29b-41d4-a716-446655440001",
    "start_time": "2025-01-15T14:00:00Z",
    "end_time": "2025-01-15T16:00:00Z"
  }
}
```

**Possible Errors:**
- `401` - Invalid or expired token
- `403` - Admins cannot view bookings
- `404` - Booking not found (or doesn't belong to user)

---

## Authentication & Authorization

### JWT Token Format

All protected endpoints require a Bearer token in the Authorization header:

```
Authorization: Bearer <token>
```

### Token Structure

Tokens contain the following claims:
- `id` - User UUID (admin_id or emp_id)
- `sub` - User email/identifier
- `role` - User role (Admin, Manager, or Employee)
- `exp` - Expiration timestamp (default: 1 hour, configurable via TOKEN_EXPIRY_HOURS env var)

### Token Validity

- Tokens are verified on every protected request
- Expired tokens return `401 Unauthorized`
- Invalid tokens return `401 Unauthorized`
- Missing Authorization header returns `401 Unauthorized`

---

## Error Codes & Messages

### HTTP Status Codes

| Code | Meaning | Typical Cause |
| --- | --- | --- |
| 200 | OK | Successful GET/PATCH request |
| 201 | Created | Successful POST request |
| 400 | Bad Request | Invalid input, validation failed |
| 401 | Unauthorized | Missing/invalid/expired token |
| 403 | Forbidden | Insufficient permissions for role |
| 404 | Not Found | Resource doesn't exist |
| 422 | Unprocessable Entity | Business logic validation failed |
| 500 | Internal Server Error | Database or server error |

### Common Error Messages

- `"invalid credentials"` - Login failed (wrong password or user not found)
- `"activate your credentials"` - Employee hasn't set password yet
- `"only managers have access"` - Endpoint requires Manager role
- `"only administrators have access"` - Endpoint requires Admin role
- `"only employees have access"` - Admins cannot perform this action
- `"invalid email format"` - Email doesn't contain @ or is empty
- `"account already activated"` - Employee already has password set
- `"space slot already filled"` - Booking time conflict
- `"you can only book for todays date"` - Booking must be for current day
- `"invalid timings"` - Start/end time validation failed
- `"booking time must be in the future"` - Booking start time is in the past
- `"you can only book for max 2 hours"` - Booking duration exceeds 2 hours
- `"no parameters provided"` - Update request has no fields

---

## Environment Variables

| Variable | Description | Default |
| --- | --- | --- |
| `DATABASE_URL` | PostgreSQL connection string | Required |
| `JWT_SECRET` | Secret key for JWT signing | Required |
| `SERVER_ADDRESS` | Server bind address | 127.0.0.1 |
| `PORT` | Server port | 7879 |
| `TOKEN_EXPIRY_HOURS` | JWT token expiration in hours | 1 |

---

## Usage Examples

### Example 1: Company Admin Creates Company with Manager

1. Admin logs in:
```bash
POST /auth/admin/login
{
  "email": "admin@cospace.com",
  "password": "admin123"
}
```
Returns admin token

2. Admin creates company:
```bash
POST /companies
Authorization: Bearer <admin_token>
{
  "company_name": "Tech Corp",
  "about": "Software company",
  "manager": {
    "name": "Alice Johnson",
    "position": "Director",
    "email": "alice@techcorp.com"
  }
}
```
Returns manager token

3. Manager verifies email and sets password:
```bash
PATCH /employees/{manager_id}/verify
Authorization: Bearer <manager_token>
{
  "password": "securepass123"
}
```

4. Manager logs in:
```bash
POST /auth/login/employee
{
  "email": "alice@techcorp.com",
  "password": "securepass123"
}
```

### Example 2: Manager Invites Employee and Employee Books Space

1. Manager creates employee:
```bash
POST /employees
Authorization: Bearer <manager_token>
{
  "name": "Bob Smith",
  "position": "Engineer",
  "email": "bob@techcorp.com"
}
```
Returns employee token

2. Employee verifies and sets password:
```bash
PATCH /employees/{emp_id}/verify
Authorization: Bearer <employee_token>
{
  "password": "emppass456"
}
```

3. Employee logs in:
```bash
POST /auth/login/employee
{
  "email": "bob@techcorp.com",
  "password": "emppass456"
}
```

4. Employee checks available spaces:
```bash
GET /spaces/available?start_time=2025-01-15T14:00:00Z&end_time=2025-01-15T15:00:00Z
```

5. Employee books a space:
```bash
POST /bookings
Authorization: Bearer <employee_token>
{
  "space_id": "770e8400-e29b-41d4-a716-446655440002",
  "start_time": "2025-01-15T14:00:00Z",
  "end_time": "2025-01-15T15:00:00Z"
}
```

6. Employee extends booking:
```bash
PATCH /bookings/{booking_id}
Authorization: Bearer <employee_token>
{
  "extra_time": {
    "secs": 1800,
    "nanos": 0
  }
}
```

---

## Summary of Endpoints by Role

### Admin Endpoints (19 total)
- `POST /auth/admin/login`
- `POST /companies`
- `GET /companies/{id}`
- `GET /companies`
- `PATCH /companies/{id}`
- `DELETE /companies/{id}`
- `POST /spaces`
- `PATCH /spaces/{id}`
- `DELETE /spaces/{id}`
- `GET /spaces` (public)
- `GET /spaces/{id}` (public)
- `GET /spaces/available` (public)
- `GET /spaces/{id}/bookings` (public)

### Manager Endpoints (12 total, includes shared)
- `POST /auth/login/employee`
- `GET /me/company`
- `POST /employees`
- `GET /employees/{id}`
- `GET /employees`
- `PATCH /employees/{id}`
- `DELETE /employees/{id}`
- `POST /bookings`
- `DELETE /bookings/{id}`
- `PATCH /bookings/{id}`
- `GET /bookings/company`
- `GET /bookings/me`

### Employee Endpoints (8 total, includes shared)
- `POST /auth/login/employee`
- `PATCH /employees/{id}/verify`
- `GET /me/company`
- `POST /bookings`
- `DELETE /bookings/{id}`
- `PATCH /bookings/{id}`
- `GET /bookings/me`
- `GET /bookings/{id}`

### Public Endpoints (3 total)
- `GET /spaces`
- `GET /spaces/{id}`
- `GET /spaces/available`
- `GET /spaces/{id}/bookings`

---

## Summary

The Co-Space API provides a complete workspace management system with:
- **3 user roles** with distinct responsibilities
- **27 total endpoints** covering authentication, company, employee, space, and booking management
- **Role-based access control** enforcing security policies
- **Time-based constraints** for space bookings (2-hour max, today only)
- **Conflict prevention** preventing double-booking through overlapping checks
- **JWT token authentication** for all protected operations
