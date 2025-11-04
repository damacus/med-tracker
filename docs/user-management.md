# User Management System

## Overview

The Med Tracker application now includes a comprehensive user management system with multiple access levels and capacity management. The system separates the concept of a **Person** (any individual in the system) from a **User** (someone who can log in).

## Core Concepts

### Person vs User Separation

- **Person**: Any individual in the system, whether they can log in or not
  - Children who receive medication
  - Adults with capacity issues
  - Carers, nurses, doctors, administrators
  
- **User**: Authentication entity for people who need to log in
  - Optional - not all people need user accounts
  - Linked one-to-one with a Person record

### Person Types

The system supports five person types:

1. **Patient** (default)
   - People receiving medication
   - May or may not have capacity
   - Can have multiple carers assigned

2. **Carer**
   - Can record medicines for assigned patients
   - Can manage multiple patients
   - Typically parents or guardians

3. **Nurse**
   - Professional carer with additional permissions
   - Can record medications
   - Can view assigned patients

4. **Doctor**
   - Can prescribe medications
   - Can view medical records
   - Higher level of access

5. **Administrator**
   - Full system access
   - Can manage users and people
   - System configuration access

### User Roles

For people who can log in, the system supports five roles:

- `administrator` - Full system access
- `doctor` - Can prescribe, view medical records
- `nurse` - Can record medications, view assigned patients
- `carer` - Can record medications for assigned patients
- `parent` - Special carer type with family relationship

### Capacity Management

The system tracks whether a person has legal capacity to make their own decisions:

- `has_capacity: true` (default) - Person can make their own decisions
- `has_capacity: false` - Person requires a nominated carer

People without capacity:

- Must have at least one carer assigned
- Cannot have their own user account for medication management
- All medication administration must be recorded by their carer

### Carer Relationships

The system supports many-to-many relationships between carers and patients:

- A patient can have multiple carers
- A carer can manage multiple patients
- Each relationship has a `relationship_type` (e.g., "parent", "guardian", "professional_carer")
- Relationships can be marked as `active` or inactive

## Database Schema

### People Table

```ruby
create_table "people" do |t|
  t.string "name", null: false
  t.string "email"
  t.date "date_of_birth"
  t.integer "person_type", default: 0, null: false  # 0=patient, 1=carer, 2=nurse, 3=doctor, 4=administrator
  t.boolean "has_capacity", default: true, null: false
  t.timestamps
end
```

### Users Table

```ruby
create_table "users" do |t|
  t.string "email_address", null: false
  t.string "password_digest", null: false
  t.integer "role", default: 0, null: false  # 0=administrator, 1=doctor, 2=nurse, 3=carer, 4=parent
  t.integer "person_id", null: false
  t.timestamps
end
```

### Carer Relationships Table

```ruby
create_table "carer_relationships" do |t|
  t.integer "carer_id", null: false  # references people
  t.integer "patient_id", null: false  # references people
  t.string "relationship_type"
  t.boolean "active", default: true, null: false
  t.timestamps
end
```

## Model Relationships

### Person Model

```ruby
class Person < ApplicationRecord
  # Authentication
  has_one :user
  
  # Medications
  has_many :prescriptions
  has_many :medicines, through: :prescriptions
  has_many :person_medicines
  
  # Carer relationships
  has_many :carer_relationships, foreign_key: :patient_id
  has_many :carers, through: :carer_relationships
  
  # Patient relationships (people this person cares for)
  has_many :patient_relationships, class_name: 'CarerRelationship', foreign_key: :carer_id
  has_many :patients, through: :patient_relationships
  
  enum :person_type, { patient: 0, carer: 1, nurse: 2, doctor: 3, administrator: 4 }
end
```

### User Model

```ruby
class User < ApplicationRecord
  belongs_to :person
  has_secure_password
  has_many :sessions
  
  enum :role, { administrator: 0, doctor: 1, nurse: 2, carer: 3, parent: 4 }
end
```

### CarerRelationship Model

```ruby
class CarerRelationship < ApplicationRecord
  belongs_to :carer, class_name: 'Person'
  belongs_to :patient, class_name: 'Person'
  
  scope :active, -> { where(active: true) }
  
  def deactivate!
    update!(active: false)
  end
  
  def activate!
    update!(active: true)
  end
end
```

## Usage Examples

### Creating a Patient Without Capacity

```ruby
# Create a child patient
child = Person.create!(
  name: 'Emma Smith',
  date_of_birth: 5.years.ago,
  person_type: :patient,
  has_capacity: false
)

# Create a parent carer
parent = Person.create!(
  name: 'Sarah Smith',
  date_of_birth: 35.years.ago,
  person_type: :carer
)

# Create user account for parent
user = User.create!(
  email_address: 'sarah@example.com',
  password: 'secure_password',
  person: parent,
  role: :parent
)

# Link parent as carer for child
child.carer_relationships.create!(
  carer: parent,
  relationship_type: 'parent'
)
```

### Creating a Nurse

```ruby
# Create person record
nurse_person = Person.create!(
  name: 'Jane Nurse',
  date_of_birth: 30.years.ago,
  person_type: :nurse
)

# Create user account
nurse_user = User.create!(
  email_address: 'jane.nurse@hospital.com',
  password: 'secure_password',
  person: nurse_person,
  role: :nurse
)
```

### Assigning Multiple Carers

```ruby
patient = Person.find_by(name: 'Emma Smith')

# Add professional carer
professional = Person.create!(
  name: 'Professional Carer',
  date_of_birth: 40.years.ago,
  person_type: :carer
)

patient.carer_relationships.create!(
  carer: professional,
  relationship_type: 'professional_carer'
)

# Patient now has multiple carers
patient.carers.count # => 2
```

### Checking Permissions

```ruby
user = User.find_by(email_address: 'jane.nurse@hospital.com')

user.nurse? # => true
user.administrator? # => false
user.person.nurse? # => true
```

## Testing

Comprehensive test coverage includes:

- **Person model tests**: person types, capacity, carer relationships
- **User model tests**: roles, validations, authentication
- **CarerRelationship model tests**: associations, validations, scopes

Run tests with:

```bash
bundle exec rspec spec/models/person_spec.rb
bundle exec rspec spec/models/user_spec.rb
bundle exec rspec spec/models/carer_relationship_spec.rb
```

## Fixtures

Test fixtures are available for all person types and user roles:

- `people.yml`: Includes patients, carers, nurses, doctors, administrators
- `users.yml`: Includes all user roles
- `carer_relationships.yml`: Sample carer-patient relationships

## Next Steps

Future enhancements could include:

1. **Authorization Layer**: Implement Pundit or similar for role-based permissions
2. **Admin Interface**: Create UI for managing users and people
3. **Audit Logging**: Track who records medications for whom
4. **Notifications**: Alert carers when medications are due
5. **Multi-tenancy**: Support for multiple organizations/facilities
6. **API Access**: RESTful API for mobile apps

## Migration History

1. `20250930215217_add_person_type_and_capacity_to_people.rb` - Added person_type and has_capacity columns
2. `20250930220140_create_carer_relationships.rb` - Created carer_relationships table
3. `20250930220454_fix_person_type_and_capacity_defaults.rb` - Fixed default values and constraints

## Security Considerations

- Passwords are securely hashed using bcrypt
- Email addresses are normalized (lowercased, trimmed)
- Person-User relationship is one-to-one
- Carer-Patient relationships are validated for uniqueness
- All sensitive operations should check user permissions
