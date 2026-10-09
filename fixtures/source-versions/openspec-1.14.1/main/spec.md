# Session Specification

## Purpose
Describe how the application authenticates a user and protects sessions from incorrect credentials.

## Requirements

### Requirement: Password authentication
The application SHALL create a session only after the supplied password matches the stored credential.

#### Scenario: Correct password
- **WHEN** a user supplies the correct password
- **THEN** the application creates an authenticated session

#### Scenario: Incorrect password
- **WHEN** a user supplies an incorrect password
- **THEN** the application rejects the login without creating a session
