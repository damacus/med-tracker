# Medicine Tracker Application Design

## Overview

A Rails-based application for tracking medicine administration, including dosages, people, and timing. The application uses OpenID Connect (OIDC) for authentication, currently implemented via Google OAuth 2.0.

## Core Features

### Authentication

- **OIDC-based authentication** (implemented via Google OAuth 2.0)
- Email/password authentication via Rodauth
- User management and authorization
- Secure session handling
- Two-factor authentication (TOTP)

### Data Models

#### User

- Basic user information from OIDC provider (Google) or email signup
- Role-based access control (administrator, doctor, nurse, carer, parent, minor)
- Relationships to people they manage (e.g., family members)
- Authentication via Rodauth (email/password or OIDC)

#### Person

- Name
- Date of birth
- Relationships to medicines
- Associated users (caretakers/family members)

#### Medicine

- Name
- Description
- Active status
- Standard dosage information
- Warnings/Notes

#### Prescription

- Links Person to Medicine
- Specific dosage for this person
- Frequency of administration
- Start date
- End date (optional)
- Special instructions

#### DosageRecord

- Timestamp of administration
- Link to Prescription
- Actual dose given
- Administered by (User)
- Notes

## Technical Architecture

### Backend

- Ruby on Rails (8.1+) with Hotwire (Turbo + Stimulus)
- SQLite3 database (development), PostgreSQL (production recommended)
- OIDC integration via Rodauth and OmniAuth
- API versioning for future compatibility

### Security Considerations

- **OIDC authentication** with ID token verification (signature, issuer, audience, expiration)
- **Email/password authentication** with bcrypt hashing and account lockout
- **Two-factor authentication** (TOTP) via Rodauth
- Role-based access control via Pundit policies
- Audit logging for all medicine administration via PaperTrail
- Secure data storage following healthcare data best practices
- CSRF protection via Rails authenticity tokens
- Session management with Rodauth (active sessions, remember me)
- Security headers (CSP, HSTS, X-Frame-Options, etc.)
- Regular security updates and dependency scanning

### Database Considerations

- Data integrity constraints
- Indexing for common queries
- Audit trail for all changes

## Development Practices

- End-to-End Testing with Playwright
  - Full browser automation testing
  - Cross-browser testing capabilities
  - Automatic waiting and retry mechanisms
  - Test recording and debugging tools
- Code linting and formatting
- Comprehensive documentation
- Git version control
- GitHub Actions for CI/CD
  - Automated Playwright tests in CI pipeline
  - Cross-browser testing in CI
- Code review process

## Future Considerations

- Mobile application support
- Medication schedule reminders
- Integration with pharmacy systems
- Export/reporting capabilities
- Multiple timezone support
- Offline capabilities

## Next Steps

1. ✅ Set up basic Rails application structure
2. ✅ Implement OIDC authentication (Google OAuth 2.0)
3. ✅ Implement email/password authentication (Rodauth)
4. ✅ Create core data models (Person, Medicine, Prescription, MedicationTake)
5. ✅ Implement basic CRUD operations
6. ✅ Add authorization layer (Pundit)
7. ✅ Create user interface (Phlex components with Hotwire)
8. ✅ Implement audit logging (PaperTrail)
9. ✅ Add automated tests (RSpec + Capybara)
10. ✅ Set up CI/CD pipeline (GitHub Actions)
11. 🚧 Enhanced features (2FA, additional OIDC providers, mobile optimization)
