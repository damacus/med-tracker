# Medicine Tracker Application Design

## Overview
A Rails-based application for tracking medicine administration, including dosages, people, and timing. The application will use OpenID Connect (OIDC) for authentication.

## Core Features

### Authentication
- OIDC-based authentication (provider TBD)
- User management and authorization
- Secure session handling

### Data Models

#### User
- Basic user information from OIDC provider
- Role-based access control
- Relationships to people they manage (e.g., family members)

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
- Ruby on Rails API
- SQLite3 database
- OIDC integration
- API versioning for future compatibility

### Security Considerations
- OIDC authentication
- Role-based access control
- Audit logging for all medicine administration
- Secure data storage following healthcare data best practices
- CSRF protection
- Regular security updates

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
1. Set up basic Rails application structure
2. Implement OIDC authentication
3. Create core data models
4. Implement basic CRUD operations
5. Add authorization layer
6. Create user interface
7. Implement audit logging
8. Add automated tests
9. Set up CI/CD pipeline
