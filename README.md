# MakeMeShort API

---

MakeMeShort is a powerful URL shortening service with advanced features including QR code generation and comprehensive analytics.

## Table of Contents

---

- [Base URL](#base-url)
- [Authentication Requirements](#authentication-requirements)
- [Endpoints](#endpoints)
  - [Authentication](#authentication)
  - [Role-Based Access Control](#role-based-access-control)
  - [URL Operations](#url-operations)
  - [QR Code Operations](#qr-code-operations)
  - [Analytics](#analytics)
  - [System Operations](#system-operations)
- [Error Responses](#error-responses)
- [Data Models](#data-models)

## Base URL

---

```
localhost:8080/api/
```

## Authentication Requirements

---

All API endpoints except `/auth/*` and `/health/check` require authentication using a JWT token in the Authorization header:

```
Authorization: Bearer <your_jwt_token>
```

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

---

Authenticate and receive a JWT token.

```
URL: /api/auth/login
Method: POST
```

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
  "username": "your_username",
  "roles": ["URL Creator", "Analytics Viewer"]
}
```

### URL Operations

---

#### Create Short URL

---

Creates a shortened URL for a given original URL.

```
URL: /shorten
Method: POST
Headers:
  - Content-Type: application/json
  - Authorization: Bearer <token>
```

**Request Body:**

```json
{
  "url": "https://example.com/very/long/url/that/needs/shortening",
  "expires_in_days": 7 // Optional: expiration in days
}
```

**Response:**

```json
{
  "original_url": "https://example.com/very/long/url/that/needs/shortening",
  "short_url": "https://mms.io/abc123",
  "short_code": "abc123",
  "expires_at": 1649289600000 // Timestamp in milliseconds, null if no expiration
}
```

#### List All URLs

---

Lists all shortened URLs with optional search functionality.

```
URL: /urls
Method: GET
Headers:
  - Authorization: Bearer <token>
```

**Query Parameters:**

- `search` - Optional search term to filter URLs

**Response:**

```json
[
  {
    "id": "67ed3b7ff4867055144c4759",
    "original_url": "https://example.com/very/long/url/that/needs/shortening",
    "short_code": "Wp0IEE",
    "created_at": 1743600511583,
    "expires_at": 1744205311583,
    "has_shortened_qr": false,
    "has_original_qr": false,
    "clicks": 0,
    "unique_clicks": 0
  }
]
```

#### Redirect to Original URL

---

Redirects to the original URL and tracks the click.

```
URL: /r/{code}
Method: GET
Headers:
  - Authorization: Bearer <token>
```

**Path Parameters:**

- `code` - The short code of the URL

**Response:** HTTP 302 redirect to the original URL

### QR Code Operations

---

#### Regenerate QR Code

---

Regenerates a QR code for a URL.

```
URL: /qr/{code}/regenerate
Method: GET
Headers:
  - Authorization: Bearer <token>
```

**Path Parameters:**

- `code` - The short code of the URL

**Query Parameters:**

- `force` - Whether to force regeneration (optional, default: false)
- `url_type` - Type of URL to encode (optional, default: "shortened")

**Response:** SVG image of the QR code (Content-Type: image/svg+xml)

#### Get QR Code Info

---

Gets the QR code SVG directly.

```
URL: /qr/{code}/info
Method: GET
Headers:
  - Authorization: Bearer <token>
```

**Path Parameters:**

- `code` - The short code of the URL

**Query Parameters:**

- `url_type` - Type of URL to retrieve (optional, default: "shortened")

**Response:** SVG image of the QR code (Content-Type: image/svg+xml)

#### Generate QR Code Directly

---

Generates a QR code for any URL without requiring it to be shortened first.

```
URL: /qr
Method: POST
Headers:
  - Content-Type: application/json
  - Authorization: Bearer <token>
```

**Request Body:**

```json
{
  "url": "https://example.com",
  "size": 300, // Optional, default is 200
  "force_regenerate": false // Optional, default is false
}
```

**Response:** SVG image of the QR code (Content-Type: image/svg+xml)

### Analytics

---

#### Get URL Analytics

---

Gets analytics for a specific URL.

```
URL: /analytics/{code}
Method: GET
Headers:
  - Authorization: Bearer <token>
```

**Path Parameters:**

- `code` - The short code of the URL

**Response:**

```json
{
  "short_code": "Wp0IEE",
  "original_url": "https://google.com",
  "created_at": 1743600511583,
  "expires_at": 1744205311583,
  "clicks": 1,
  "unique_clicks": 1,
  "has_shortened_qr": false,
  "has_original_qr": false,
  "shortened_qr_generated_at": null,
  "original_qr_generated_at": null
}
```

### System Operations

---

#### Health Check

---

Checks if the API is running.

```
URL: /health/check
Method: GET
```

**Response:**

```json
{
  "status": "ok",
  "version": "1.0.0",
  "timestamp": 1649203200000
}
```

## Error Responses

---

### Common Error Responses

---

- **400 Bad Request**: Invalid request parameters
- **404 Not Found**: URL not found
- **410 Gone**: URL has expired
- **500 Internal Server Error**: Server error

Example error response:

```json
{
  "error": {
    "code": "URL_NOT_FOUND",
    "message": "The requested URL does not exist",
    "status": 404
  }
}
```

## Data Models

---

### ShortenedUrl

---

- `id`: ObjectId (MongoDB ID)
- `original_url`: String (The original URL)
- `short_code`: String (The short code used in the shortened URL)
- `created_at`: i64 (Timestamp in milliseconds)
- `expires_at`: Optional<i64> (Expiration timestamp in milliseconds)
- `clicks`: i64 (Number of clicks on the shortened URL)

### QrCode

---

- `id`: ObjectId (MongoDB ID)
- `short_code`: String (Reference to the shortened URL)
- `original_url`: String (The original URL)
- `svg_content`: String (SVG content of the QR code)
- `generated_at`: i64 (Timestamp in milliseconds)
- `target_type`: String ("original" or "shortened")

### UrlVisitor

---

- `id`: ObjectId (MongoDB ID)
- `short_code`: String (Reference to the shortened URL)
- `visitor_hash`: String (Hashed IP address of the visitor)
- `timestamp`: i64 (Timestamp in milliseconds)
- `user_agent`: Optional<String> (User agent of the visitor)
- `referrer`: Optional<String> (Referrer of the visitor)
