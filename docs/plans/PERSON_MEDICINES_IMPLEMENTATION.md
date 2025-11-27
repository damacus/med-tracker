# Person Medicines Implementation

## Overview

Implemented support for tracking medicines (vitamins, supplements, OTC medications) that don't require prescriptions, while maintaining the ability to record when they're taken and enforce timing restrictions.

## Database Changes

### New Tables

- **`person_medicines`**: Join table for direct person-medicine associations
  - `person_id` (required)
  - `medicine_id` (required)
  - `notes` (optional)
  - `max_daily_doses` (optional timing restriction)
  - `min_hours_between_doses` (optional timing restriction)
  - Unique index on `[person_id, medicine_id]`

### Modified Tables

- **`medication_takes`**: Updated to support both prescriptions and person_medicines
  - `prescription_id` (now nullable)
  - `person_medicine_id` (new, nullable)
  - Constraint: Exactly one of `prescription_id` or `person_medicine_id` must be present (enforced at model level)

## Models

### PersonMedicine

- Represents direct person-medicine associations without prescriptions
- Includes timing restriction logic:
  - `can_take_now?` - Determines if medicine can be taken based on restrictions
  - `next_available_time` - Calculates when medicine can next be taken
  - `timing_restrictions?` - Checks if any restrictions are set
- Associations:
  - `belongs_to :person`
  - `belongs_to :medicine`
  - `has_many :medication_takes`

### MedicationTake (Updated)

- Now supports both prescription-based and non-prescription medicines
- `belongs_to :prescription, optional: true`
- `belongs_to :person_medicine, optional: true`
- Validation ensures exactly one source is present
- Helper methods:
  - `source` - Returns prescription or person_medicine
  - `person` - Returns associated person
  - `medicine` - Returns associated medicine

### Person (Updated)

- Added associations:
  - `has_many :person_medicines`
  - `has_many :non_prescription_medicines, through: :person_medicines`

### Medicine (Updated)

- Added association:
  - `has_many :person_medicines`

## Controllers

### PersonMedicinesController

- `new` - Display form to add medicine (supports Turbo Streams for modal)
- `create` - Add medicine to person
- `destroy` - Remove medicine from person
- `take_medicine` - Record taking a medicine

Routes:

```ruby
resources :people do
  resources :person_medicines, except: [:index] do
    member do
      post :take_medicine
    end
  end
end
```

## UI Components

### Components::PersonMedicines::Card

- Displays medicine information
- Shows timing restrictions if present
- Lists today's doses
- **Take button with disabled state** when timing restrictions prevent taking
- Shows next available time when button is disabled
- Delete confirmation dialog

### Components::People::ShowView (Updated)

- Added "My Medicines" section below "Prescriptions"
- Displays grid of person medicine cards
- "Add Medicine" button to add new medicines
- Empty state with helpful text

## Features

### Timing Restrictions

- **Max daily doses**: Limits number of times medicine can be taken per day
- **Min hours between doses**: Enforces minimum time between doses
- Button automatically disables when restrictions prevent taking
- Shows when medicine will next be available

### Button Disabled State

The "Take" button uses the pattern:

```ruby
Button(disabled: !person_medicine.can_take_now?)
```

This ensures users cannot take medicine when:

1. Maximum daily doses have been reached
2. Minimum hours between doses haven't passed

## Internationalization

Added translations in `config/locales/en.yml`:

```yaml
person_medicines:
  created: "Medicine added successfully."
  deleted: "Medicine removed successfully."
  medicine_taken: "Medicine taken successfully."
```

## Testing

### System Tests

- `spec/features/person_medicines_spec.rb` - Feature tests for:
  - Adding non-prescription medicines
  - Recording medicine takes
  - Disabled button when max doses reached
  - Disabled button when min hours not passed
  - Viewing medication history

### Model Tests

- `spec/models/medication_take_spec.rb` - Updated to test:
  - Optional prescription and person_medicine associations
  - Exactly one source validation
  - Source delegation methods

## Terminology

- **Prescriptions**: Formal prescriptions with dosages and schedules
- **My Medicines**: Vitamins, supplements, and OTC medicines without prescriptions

## Next Steps (Not Yet Implemented)

1. **Medication History Table**: Create a table component using RubyUI Table to display all medication takes with filtering
2. **Form Views**: Create the actual form partial for adding person medicines
3. **Turbo Stream Responses**: Implement modal behavior for add/edit forms
4. **Amount Tracking**: Add ability to specify amount when taking medicine (currently uses default)
5. **Edit Functionality**: Add ability to edit person medicine settings

## Usage Example

```ruby
# Create a person medicine
person_medicine = PersonMedicine.create!(
  person: person,
  medicine: vitamin_d,
  notes: 'Take with breakfast',
  max_daily_doses: 1,
  min_hours_between_doses: 24
)

# Check if can take now
person_medicine.can_take_now? # => true/false

# Get next available time
person_medicine.next_available_time # => Time object or nil

# Record taking the medicine
person_medicine.medication_takes.create!(
  taken_at: Time.current,
  amount_ml: 5
)
```
