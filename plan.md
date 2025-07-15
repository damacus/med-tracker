# MEDTRACKER MIGRATION TO BRUTRB

## Architecture Migration Plan

### Phase 1: Framework Transition

- **BREAKING CHANGE**: Migrate from Ruby on Rails to BrutRB framework
- Replace Rails MVC architecture with BrutRB's approach
- Maintain core medication tracking functionality during transition
- Preserve existing database schema where possible

### Phase 2: BrutRB Implementation

#### Core Application Structure

- Rebuild controllers using BrutRB conventions
- Implement BrutRB routing and middleware
- Convert Phlex components to BrutRB-compatible views
- Migrate authentication system to work with BrutRB

#### Data Layer

- Adapt existing ActiveRecord models to BrutRB's data handling
- Maintain current database schema (users, medicines, prescriptions, medication_takes)
- Preserve data integrity and relationships
- Keep existing validations and business logic

#### Frontend Integration

- Evaluate BrutRB's frontend capabilities vs current Hotwire setup
- Maintain responsive design and user experience
- Preserve existing CSS and styling where compatible
- Adapt Stimulus controllers if needed

### Phase 3: Feature Preservation

#### Medicine Supply Management

- Order tablets tracking
- Supply level monitoring
- Lead time management
- Low stock alerts

#### Medicine Catalog

- Laxido
- Movicol
- Vitamins (adult)
- Vitamins (child)
- Custom medicine additions

#### Administration & Logging

- Record who administered medicine (default to logged in user)
- Comprehensive log book / record of medicine taken
- Timestamp tracking for all doses
- Dose validation and safety checks

#### User Management & Permissions

**Child Role:**

- Can take medicine
- Cannot order medicine
- Cannot see other users
- Cannot see other medicines
- Cannot see other orders

**Carer Role:**

- Can take medicine
- Can order medicine
- Can see other users
- Cannot see other medicines
- Cannot see other orders
- Cannot see admin functions
- Cannot see other roles

**Admin Role:**

- Full system access
- Can take medicine
- Can order medicine
- Can see and manage all users
- Can see and manage all medicines
- Can see and manage all orders
- Can manage admin functions
- Can manage user roles
- Full CRUD operations on all entities
- Can assign users to roles
- Can assign medicines to users
- Can assign orders to users

#### Notifications System

- Medicine taking reminders
- Medicine ordering alerts
- Low stock notifications
- Dose timing warnings

### Phase 4: Testing & Quality Assurance

- Migrate existing RSpec tests to work with BrutRB
- Maintain Playwright end-to-end tests
- Ensure all safety validations work correctly
- Test role-based permissions thoroughly
- Validate data migration integrity

### Phase 5: Deployment

- Update deployment configuration for BrutRB
- Modify Docker setup if needed
- Update CI/CD pipeline
- Plan zero-downtime migration strategy

## Migration Risks & Considerations

- **Data Safety**: Critical to preserve all medication records
- **User Experience**: Minimize disruption to daily medication tracking
- **Security**: Maintain role-based access controls
- **Performance**: Ensure BrutRB performs well for medication timing
- **Testing**: Comprehensive testing of safety-critical features

## Success Criteria

- All existing functionality preserved
- Improved performance with BrutRB
- Maintained data integrity
- User roles and permissions working correctly
- All safety validations operational
- Successful deployment with zero data loss
