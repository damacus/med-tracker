# Database Relationships

This diagram focuses on the core medication and care domain tables.

```mermaid
erDiagram
    ACCOUNTS {
      bigint id PK
      citext email
      integer status
    }

    PEOPLE {
      bigint id PK
      bigint account_id FK
      string name
      string email
      date date_of_birth
      integer person_type
      boolean has_capacity
    }

    USERS {
      bigint id PK
      bigint person_id FK
      string email_address
      integer role
      boolean active
    }

    MEDICATIONS {
      bigint id PK
      string name
      integer current_supply
    }

    DOSAGES {
      bigint id PK
      bigint medication_id FK
      decimal amount
      string unit
    }

    SCHEDULES {
      bigint id PK
      bigint person_id FK
      bigint medication_id FK
      bigint dosage_id FK
      integer max_daily_doses
      integer min_hours_between_doses
      date start_date
      date end_date
      boolean active
    }

    PERSON_MEDICATIONS {
      bigint id PK
      bigint person_id FK
      bigint medication_id FK
      integer max_daily_doses
      integer min_hours_between_doses
    }

    MEDICATION_TAKES {
      bigint id PK
      bigint schedule_id FK
      bigint person_medication_id FK
      datetime taken_at
    }

    CARER_RELATIONSHIPS {
      bigint id PK
      bigint carer_id FK
      bigint patient_id FK
      string relationship_type
      boolean active
    }

    ACCOUNTS ||--o| PEOPLE : "linked profile"
    PEOPLE ||--o| USERS : "login account"
    MEDICATIONS ||--o{ DOSAGES : "has"
    PEOPLE ||--o{ SCHEDULES : "has"
    PEOPLE ||--o{ PERSON_MEDICATIONS : "has"
    MEDICATIONS ||--o{ SCHEDULES : "scheduled in"
    MEDICATIONS ||--o{ PERSON_MEDICATIONS : "assigned in"
    DOSAGES ||--o{ SCHEDULES : "used by"
    SCHEDULES ||--o{ MEDICATION_TAKES : "records"
    PERSON_MEDICATIONS ||--o{ MEDICATION_TAKES : "records"
    PEOPLE ||--o{ CARER_RELATIONSHIPS : "as carer"
    PEOPLE ||--o{ CARER_RELATIONSHIPS : "as patient"
```

## Notes

- `MedicationTake` must reference exactly one source (`schedule_id` or
  `person_medication_id`) at the model layer.
- Carer relationships are self-referential links on `people`.
