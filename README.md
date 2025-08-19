# MakeMeShort API

---

MakeMeShort is a powerful URL shortening service with advanced features including QR code generation and comprehensive analytics.

## Table of Contents

---

- [Base URL](#base-url)
- [Authentication Requirements](#authentication-requirements)
- [Endpoints](#endpoints)
  - [Authentication](#authentication)
  - [User Management](#user-management)
  - [URL Operations](#url-operations)
  - [QR Code Operations](#qr-code-operations)
  - [Analytics](#analytics)
  - [System Operations](#system-operations)
- [Error Responses](#error-responses)
- [Data Models](#data-models)

## Base URL

---

```
localhost:8080
```

## Authentication Requirements

---

All API endpoints under `/api/*` require authentication using a JWT token in the Authorization header, with the following exceptions:
- `/api/auth/login`
- `/api/auth/signup`
- `/api/auth/init`
- `/api/health/check`

The redirect endpoint `/r/{code}` also does not require authentication.

Example:

```bash
curl --location --request GET 'localhost:8080/api/urls' \
--header 'Content-Type: application/json' \
--header 'Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...'
```

## Endpoints

---

### Authentication

---

#### Login

Authenticate and receive a JWT token.

- **URL:** `/api/auth/login`
- **Method:** `POST`

**Request Body:**

```json
{
  "username": "your_username",
  "password": "your_password"
}
```

**Response:**

```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "user": {
    "id": "67f146cf3a65e380392cee79",
    "username": "your_username",
    "email": "user@example.com",
    "full_name": "Your Name",
    "created_at": 1743865551000,
    "updated_at": 1743865551000,
    "last_login": 1743865600000,
    "is_active": true
  }
}
```

#### Signup

Register a new user account. This must be enabled by setting `ALLOW_PUBLIC_SIGNUP=true` in the `.env` file.

- **URL:** `/api/auth/signup`
- **Method:** `POST`

**Request Body:**

```json
{
  "username": "newuser",
  "email": "user@example.com",
  "full_name": "New User",
  "password": "securepassword123"
}
```

**Response:** (Same format as Login response)

#### Create Initial Superuser

Creates the first administrative user. This endpoint only works if there are no other users in the database.

- **URL:** `/api/auth/init`
- **Method:** `POST`

**Request Body:**

```json
{
  "username": "admin",
  "password": "supersecretpassword"
}
```

**Response:**

```json
{
  "message": "Superuser created successfully"
}
```

### User Management

All endpoints require authentication. Access to specific user resources is protected by ownership checks.

---

#### List All Users

Lists all users except the currently authenticated user.

- **URL:** `/api/users`
- **Method:** `GET`

#### Create User

Creates a new user. Typically an admin-only action.

- **URL:** `/api/users`
- **Method:** `POST`

**Request Body:**

```json
{
  "username": "anotheruser",
  "email": "another@example.com",
  "full_name": "Another User",
  "password": "password123"
}
```

#### Get User Details

Retrieves details for a specific user.

- **URL:** `/api/users/{user_id}`
- **Method:** `GET`

#### Update User

Updates a specific user's details.

- **URL:** `/api/users/{user_id}`
- **Method:** `PUT`

**Request Body:**

```json
{
  "full_name": "Updated Name",
  "is_active": false
}
```

#### Delete User

Deletes a specific user.

- **URL:** `/api/users/{user_id}`
- **Method:** `DELETE`

### URL Operations

---

#### Create Short URL

Creates a shortened URL for a given original URL.

- **URL:** `/api/shorten`
- **Method:** `POST`

**Request Body:**

```json
{
  "url": "https://example.com/very/long/url/that/needs/shortening",
  "custom_code": "my-link", // Optional
  "expires_in_days": 7 // Optional
}
```

**Response:**

```json
{
  "original_url": "https://example.com/very/long/url/that/needs/shortening",
  "short_url": "http://localhost:8080/r/my-link",
  "short_code": "my-link",
  "expires_at": 1744479600000,
  "user_id": "67f146cf3a65e380392cee79"
}
```

#### List All URLs

Lists all shortened URLs with optional filters.

- **URL:** `/api/urls`
- **Method:** `GET`

**Query Parameters:**

- `search` (string): Optional search term to filter URLs by original URL or short code.
- `owned_only` (boolean): Set to `true` to show only URLs owned by the current user.
- `user_id` (string): Optional user ID to filter URLs by a specific owner (overrides `owned_only`).

#### List User's URLs

Lists all shortened URLs for a specific user.

- **URL:** `/api/users/{user_id}/urls`
- **Method:** `GET`

#### Delete Short URL

Deletes a specific shortened URL and all its associated data (QR codes, analytics). Only the owner of the URL can perform this action.

- **URL:** `/api/urls/{code}`
- **Method:** `DELETE`

**Path Parameters:**

- `code` (string, required): The short code of the URL to delete.

**Success Response:**

- `204 No Content` on successful deletion.

**Error Responses:**

- `403 Forbidden`: If the authenticated user is not the owner of the URL.
- `404 Not Found`: If no URL with the given short code exists.

#### Redirect to Original URL

Redirects to the original URL and tracks the click. Does not require authentication.

- **URL:** `/r/{code}`
- **Method:** `GET`

### QR Code Operations

---

#### List All QR Codes

Lists all generated QR codes with optional filters.

- **URL:** `/api/qr`
- **Method:** `GET`

**Query Parameters:**

- `search` (string): Optional search term.
- `target_type` (string): Filter by `original` or `shortened`.
- `direct_only` (boolean): Set to `true` to show only direct QR codes.
- `owned_only` (boolean): Set to `true` to show only QR codes owned by the current user.
- `user_id` (string): Optional user ID to filter by a specific owner.

#### List User's QR Codes

Lists all QR codes for a specific user.

- **URL:** `/api/users/{user_id}/qr`
- **Method:** `GET`

#### Generate QR Code Directly

Generates a QR code for any URL without requiring it to be shortened first.

- **URL:** `/api/qr`
- **Method:** `POST`

**Request Body:**

```json
{
  "url": "https://example.com",
  "size": 300,
  "force_regenerate": false
}
```

**Response:** SVG image of the QR code (`Content-Type: image/svg+xml`).

#### Get QR Code Info

Gets the QR code SVG directly for a shortened URL.

- **URL:** `/api/qr/{code}/info`
- **Method:** `GET`

**Query Parameters:**

- `url_type` (string): `original` or `shortened` (default).

#### Regenerate QR Code

Regenerates a QR code for a shortened URL.

- **URL:** `/api/qr/{code}/regenerate`
- **Method:** `GET`

### Analytics

---

#### Get URL Analytics

Gets analytics for a specific URL.

- **URL:** `/api/analytics/{code}`
- **Method:** `GET`

**Response:**

```json
{
  "short_code": "my-link",
  "original_url": "https://google.com",
  "created_at": 1743863649612,
  "expires_at": null,
  "clicks": 1,
  "unique_clicks": 1,
  "has_shortened_qr": true,
  "has_original_qr": false,
  "shortened_qr_generated_at": 1743863700000,
  "original_qr_generated_at": null,
  "user_id": "67f146cf3a65e380392cee79"
}
```

### System Operations

---

#### Health Check

Checks if the API and database connection are running.

- **URL:** `/api/health/check`
- **Method:** `GET`

**Response:**

```json
{
  "success": true
}
```

## Error Responses

---

- **400 Bad Request**: Invalid request parameters.
- **401 Unauthorized**: Authentication failed or token is invalid.
- **403 Forbidden**: Authenticated user does not have permission.
- **404 Not Found**: Resource not found.
- **410 Gone**: URL has expired.
- **500 Internal Server Error**: Server error.

## Data Models

---

### User

- `id`: ObjectId (MongoDB ID)
- `username`: String
- `email`: Optional<String>
- `full_name`: Optional<String>
- `created_at`: i64 (Timestamp in milliseconds)
- `updated_at`: i64 (Timestamp in milliseconds)
- `last_login`: Optional<i64> (Timestamp in milliseconds)
- `is_active`: boolean

### ShortenedUrl

- `id`: ObjectId (MongoDB ID)
- `original_url`: String
- `short_code`: String
- `created_at`: i64 (Timestamp in milliseconds)
- `expires_at`: Optional<i64> (Timestamp in milliseconds)
- `clicks`: i64
- `user_id`: Optional<String> (ID of the user who created the URL)

### QrCode

- `id`: ObjectId (MongoDB ID)
- `short_code`: String
- `original_url`: String
- `svg_content`: String (SVG content of the QR code)
- `generated_at`: i64 (Timestamp in milliseconds)
- `target_type`: String ("original" or "shortened")
- `user_id`: Optional<String> (ID of the user who created the QR code)

### UrlVisitor

- `id`: ObjectId (MongoDB ID)
- `short_code`: String
- `visitor_hash`: String (Hashed IP address of the visitor)
- `timestamp`: i64 (Timestamp in milliseconds)
- `user_agent`: Optional<String>
- `referrer`: Optional<String>
```# MakeMeShort API

---

MakeMeShort is a powerful URL shortening service with advanced features including QR code generation and comprehensive analytics.

## Table of Contents

---

- [Base URL](#base-url)
- [Authentication Requirements](#authentication-requirements)
- [Endpoints](#endpoints)
  - [Authentication](#authentication)
  - [User Management](#user-management)
  - [URL Operations](#url-operations)
  - [QR Code Operations](#qr-code-operations)
  - [Analytics](#analytics)
  - [System Operations](#system-operations)
- [Error Responses](#error-responses)
- [Data Models](#data-models)

## Base URL

---

```
localhost:8080
```

## Authentication Requirements

---

All API endpoints under `/api/*` require authentication using a JWT token in the Authorization header, with the following exceptions:
- `/api/auth/login`
- `/api/auth/signup`
- `/api/auth/init`
- `/api/health/check`

The redirect endpoint `/r/{code}` also does not require authentication.

Example:

```bash
curl --location --request GET 'localhost:8080/api/urls' \
--header 'Content-Type: application/json' \
--header 'Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...'
```

## Endpoints

---

### Authentication

---

#### Login

Authenticate and receive a JWT token.

- **URL:** `/api/auth/login`
- **Method:** `POST`

**Request Body:**

```json
{
  "username": "your_username",
  "password": "your_password"
}
```

**Response:**

```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "user": {
    "id": "67f146cf3a65e380392cee79",
    "username": "your_username",
    "email": "user@example.com",
    "full_name": "Your Name",
    "created_at": 1743865551000,
    "updated_at": 1743865551000,
    "last_login": 1743865600000,
    "is_active": true
  }
}
```

#### Signup

Register a new user account. This must be enabled by setting `ALLOW_PUBLIC_SIGNUP=true` in the `.env` file.

- **URL:** `/api/auth/signup`
- **Method:** `POST`

**Request Body:**

```json
{
  "username": "newuser",
  "email": "user@example.com",
  "full_name": "New User",
  "password": "securepassword123"
}
```

**Response:** (Same format as Login response)

#### Create Initial Superuser

Creates the first administrative user. This endpoint only works if there are no other users in the database.

- **URL:** `/api/auth/init`
- **Method:** `POST`

**Request Body:**

```json
{
  "username": "admin",
  "password": "supersecretpassword"
}
```

**Response:**

```json
{
  "message": "Superuser created successfully"
}
```

### User Management

All endpoints require authentication. Access to specific user resources is protected by ownership checks.

---

#### List All Users

Lists all users except the currently authenticated user.

- **URL:** `/api/users`
- **Method:** `GET`

#### Create User

Creates a new user. Typically an admin-only action.

- **URL:** `/api/users`
- **Method:** `POST`

**Request Body:**

```json
{
  "username": "anotheruser",
  "email": "another@example.com",
  "full_name": "Another User",
  "password": "password123"
}
```

#### Get User Details

Retrieves details for a specific user.

- **URL:** `/api/users/{user_id}`
- **Method:** `GET`

#### Update User

Updates a specific user's details.

- **URL:** `/api/users/{user_id}`
- **Method:** `PUT`

**Request Body:**

```json
{
  "full_name": "Updated Name",
  "is_active": false
}
```

#### Delete User

Deletes a specific user.

- **URL:** `/api/users/{user_id}`
- **Method:** `DELETE`

### URL Operations

---

#### Create Short URL

Creates a shortened URL for a given original URL.

- **URL:** `/api/shorten`
- **Method:** `POST`

**Request Body:**

```json
{
  "url": "https://example.com/very/long/url/that/needs/shortening",
  "custom_code": "my-link", // Optional
  "expires_in_days": 7 // Optional
}
```

**Response:**

```json
{
  "original_url": "https://example.com/very/long/url/that/needs/shortening",
  "short_url": "http://localhost:8080/r/my-link",
  "short_code": "my-link",
  "expires_at": 1744479600000,
  "user_id": "67f146cf3a65e380392cee79"
}
```

#### List All URLs

Lists all shortened URLs with optional filters.

- **URL:** `/api/urls`
- **Method:** `GET`

**Query Parameters:**

- `search` (string): Optional search term to filter URLs by original URL or short code.
- `owned_only` (boolean): Set to `true` to show only URLs owned by the current user.
- `user_id` (string): Optional user ID to filter URLs by a specific owner (overrides `owned_only`).

#### List User's URLs

Lists all shortened URLs for a specific user.

- **URL:** `/api/users/{user_id}/urls`
- **Method:** `GET`

#### Redirect to Original URL

Redirects to the original URL and tracks the click. Does not require authentication.

- **URL:** `/r/{code}`
- **Method:** `GET`

### QR Code Operations

---

#### List All QR Codes

Lists all generated QR codes with optional filters.

- **URL:** `/api/qr`
- **Method:** `GET`

**Query Parameters:**

- `search` (string): Optional search term.
- `target_type` (string): Filter by `original` or `shortened`.
- `direct_only` (boolean): Set to `true` to show only direct QR codes.
- `owned_only` (boolean): Set to `true` to show only QR codes owned by the current user.
- `user_id` (string): Optional user ID to filter by a specific owner.

#### List User's QR Codes

Lists all QR codes for a specific user.

- **URL:** `/api/users/{user_id}/qr`
- **Method:** `GET`

#### Generate QR Code Directly

Generates a QR code for any URL without requiring it to be shortened first.

- **URL:** `/api/qr`
- **Method:** `POST`

**Request Body:**

```json
{
  "url": "https://example.com",
  "size": 300,
  "force_regenerate": false
}
```

**Response:** SVG image of the QR code (`Content-Type: image/svg+xml`).

#### Get QR Code Info

Gets the QR code SVG directly for a shortened URL.

- **URL:** `/api/qr/{code}/info`
- **Method:** `GET`

**Query Parameters:**

- `url_type` (string): `original` or `shortened` (default).

#### Regenerate QR Code

Regenerates a QR code for a shortened URL.

- **URL:** `/api/qr/{code}/regenerate`
- **Method:** `GET`

### Analytics

---

#### Get URL Analytics

Gets analytics for a specific URL.

- **URL:** `/api/analytics/{code}`
- **Method:** `GET`

**Response:**

```json
{
  "short_code": "my-link",
  "original_url": "https://google.com",
  "created_at": 1743863649612,
  "expires_at": null,
  "clicks": 1,
  "unique_clicks": 1,
  "has_shortened_qr": true,
  "has_original_qr": false,
  "shortened_qr_generated_at": 1743863700000,
  "original_qr_generated_at": null,
  "user_id": "67f146cf3a65e380392cee79"
}
```

### System Operations

---

#### Health Check

Checks if the API and database connection are running.

- **URL:** `/api/health/check`
- **Method:** `GET`

**Response:**

```json
{
  "success": true
}
```

## Error Responses

---

- **400 Bad Request**: Invalid request parameters.
- **401 Unauthorized**: Authentication failed or token is invalid.
- **403 Forbidden**: Authenticated user does not have permission.
- **404 Not Found**: Resource not found.
- **410 Gone**: URL has expired.
- **500 Internal Server Error**: Server error.

## Data Models

---

### User

- `id`: ObjectId (MongoDB ID)
- `username`: String
- `email`: Optional<String>
- `full_name`: Optional<String>
- `created_at`: i64 (Timestamp in milliseconds)
- `updated_at`: i64 (Timestamp in milliseconds)
- `last_login`: Optional<i64> (Timestamp in milliseconds)
- `is_active`: boolean

### ShortenedUrl

- `id`: ObjectId (MongoDB ID)
- `original_url`: String
- `short_code`: String
- `created_at`: i64 (Timestamp in milliseconds)
- `expires_at`: Optional<i64> (Timestamp in milliseconds)
- `clicks`: i64
- `user_id`: Optional<String> (ID of the user who created the URL)

### QrCode

- `id`: ObjectId (MongoDB ID)
- `short_code`: String
- `original_url`: String
- `svg_content`: String (SVG content of the QR code)
- `generated_at`: i64 (Timestamp in milliseconds)
- `target_type`: String ("original" or "shortened")
- `user_id`: Optional<String> (ID of the user who created the QR code)

### UrlVisitor

- `id`: ObjectId (MongoDB ID)
- `short_code`: String
- `visitor_hash`: String (Hashed IP address of the visitor)
- `timestamp`: i64