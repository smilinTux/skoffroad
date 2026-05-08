# SandK Offroad API Documentation

## Overview

The SandK Offroad backend API provides endpoints for game state management, multiplayer functionality, and user data synchronization. The API is built using the Warp framework and follows RESTful principles.

## Base URL

- Development: `http://localhost:3000`
- Production: (TBD)

## Authentication

(TBD) - Will use JWT-based authentication for secure endpoints.

## Common Response Format

All API responses follow a standard format:

```json
{
  "data": {
    // Response data specific to the endpoint
  },
  "meta": {
    // Optional metadata (pagination, etc.)
  }
}
```

Error responses:

```json
{
  "code": "ERROR_CODE",
  "message": "Human readable error message",
  "details": {
    // Optional additional error details
  }
}
```

## Available Endpoints

### Health Check

`GET /health`

Check the API server status.

**Response**
```json
{
  "status": "ok",
  "timestamp": "2024-04-17T06:11:31.649Z"
}
```

## Environment Configuration

The API server can be configured using the following environment variables:

- `BACKEND_PORT`: Server port (default: 3000)
- `BACKEND_ENV`: Environment mode (development, test, production)
- `CORS_ORIGINS`: Comma-separated list of allowed CORS origins
- `REQUEST_TIMEOUT`: Request timeout in seconds (default: 30)

## Error Codes

- `INTERNAL_ERROR`: Internal server error
- `INVALID_REQUEST`: Invalid request parameters
- `UNAUTHORIZED`: Authentication required
- `FORBIDDEN`: Permission denied
- `NOT_FOUND`: Resource not found
- `VALIDATION_ERROR`: Request validation failed

## Development Guidelines

1. All new endpoints should follow the common response format
2. Use appropriate HTTP methods (GET, POST, PUT, DELETE)
3. Include comprehensive error handling
4. Document all new endpoints in this file
5. Add test cases for new functionality
6. Follow RESTful naming conventions

## Testing

Run the test suite:

```bash
cargo test
```

For API endpoint testing, use the provided test client in `tests/mod.rs`. 