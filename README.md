# MakeMeShort API

MakeMeShort is a powerful URL shortening service with advanced features including QR code generation and comprehensive analytics.

## Table of Contents

- [Base URL](#base-url)
- [Endpoints](#endpoints)
  - [URL Operations](#url-operations)
  - [QR Code Operations](#qr-code-operations)
  - [Analytics](#analytics)
  - [System Operations](#system-operations)
- [Error Responses](#error-responses)
- [Data Models](#data-models)

## Base URL

```
localhost:8080/api/
```

## Endpoints

### URL Operations

#### Create Short URL

Creates a shortened URL for a given original URL.

```
URL: /shorten
Method: POST
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

Lists all shortened URLs with optional search functionality.

```
URL: /urls
Method: GET
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

Redirects to the original URL and tracks the click.

```
URL: /r/{code}
Method: GET
```

**Path Parameters:**

- `code` - The short code of the URL

**Response:** HTTP 302 redirect to the original URL

### QR Code Operations

#### Generate QR Code

Generates a QR code for a URL.

```
URL: /qr/{code}
Method: GET
```

**Path Parameters:**

- `code` - The short code of the URL

**Query Parameters:**

- `url_type` - Type of URL to encode (optional, default: "shortened")
  - `original` - Original URL
  - `shortened` - Shortened URL

**Response:** SVG image of the QR code (Content-Type: image/svg+xml)

#### Regenerate QR Code

Regenerates a QR code for a URL.

```
URL: /qr/{code}/regenerate
Method: GET
```

**Path Parameters:**

- `code` - The short code of the URL

**Query Parameters:**

- `force` - Whether to force regeneration (optional, default: false)
- `url_type` - Type of URL to encode (optional, default: "shortened")

**Response:** SVG image of the QR code (Content-Type: image/svg+xml)

#### Get QR Code Info

Gets the QR code SVG directly.

```
URL: /qr/{code}/info
Method: GET
```

**Path Parameters:**

- `code` - The short code of the URL

**Query Parameters:**

- `url_type` - Type of URL to retrieve (optional, default: "shortened")

**Response:** SVG image of the QR code (Content-Type: image/svg+xml)

#### Generate QR Code Directly

Generates a QR code for any URL without requiring it to be shortened first.

- **URL**: `/qr`
- **Method**: `POST`
- **Request Body**:

```json
{
  "url": "https://example.com",
  "size": 300, // Optional, default is 200
  "force_regenerate": false // Optional, default is false
}
```

**Response:** SVG image of the QR code (Content-Type: image/svg+xml)

### Analytics

#### Get URL Analytics

Gets analytics for a specific URL.

```
URL: /analytics/{code}
Method: GET
```

**Path Parameters:**

- `code` - The short code of the URL

**Response:**

```json
{
  "url": {
    "original_url": "https://example.com/page1",
    "short_url": "https://mms.io/abc123",
    "short_code": "abc123",
    "created_at": 1649203200000,
    "expires_at": 1649289600000
  },
  "clicks": 42,
  "visitors": {
    "unique": 28,
    "returning": 14
  },
  "referrers": [
    { "source": "direct", "count": 20 },
    { "source": "twitter.com", "count": 15 },
    { "source": "facebook.com", "count": 7 }
  ],
  "browsers": [
    { "name": "Chrome", "count": 25 },
    { "name": "Firefox", "count": 10 },
    { "name": "Safari", "count": 7 }
  ],
  "devices": [
    { "type": "desktop", "count": 30 },
    { "type": "mobile", "count": 12 }
  ],
  "countries": [
    { "code": "US", "count": 20 },
    { "code": "UK", "count": 8 },
    { "code": "CA", "count": 6 }
  ],
  "click_history": [
    { "date": "2023-01-01", "count": 10 },
    { "date": "2023-01-02", "count": 15 },
    { "date": "2023-01-03", "count": 17 }
  ]
}
```

### System Operations

#### Health Check

Checks if the API is running.

```
URL: /health
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

The API uses standard HTTP status codes to indicate the success or failure of a request.

### Common Error Responses

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

### ShortenedUrl

- `id`: ObjectId (MongoDB ID)
- `original_url`: String (The original URL)
- `short_code`: String (The short code used in the shortened URL)
- `created_at`: i64 (Timestamp in milliseconds)
- `expires_at`: Optional<i64> (Expiration timestamp in milliseconds)
- `clicks`: i64 (Number of clicks on the shortened URL)

### QrCode

- `id`: ObjectId (MongoDB ID)
- `short_code`: String (Reference to the shortened URL)
- `original_url`: String (The original URL)
- `svg_content`: String (SVG content of the QR code)
- `generated_at`: i64 (Timestamp in milliseconds)
- `target_type`: String ("original" or "shortened")

### UrlVisitor

- `id`: ObjectId (MongoDB ID)
- `short_code`: String (Reference to the shortened URL)
- `visitor_hash`: String (Hashed IP address of the visitor)
- `timestamp`: i64 (Timestamp in milliseconds)
- `user_agent`: Optional<String> (User agent of the visitor)
- `referrer`: Optional<String> (Referrer of the visitor)
