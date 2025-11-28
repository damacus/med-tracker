# Database Relationships

## Overview

This document describes the database relationships for the MedTracker application.

## Entity Relationship Diagram

```mermaid
erDiagram
    users {
        int id PK
        string email_address
        string password_digest
        string name
        date date_of_birth
        int role
    }
    medicines {
        int id PK
        string name
        int current_supply
    }
    dosages {
        int id PK
        int medicine_id FK
        decimal amount
        string unit
        string frequency
        string description
    }
    prescriptions {
        int id PK
        int user_id FK
        int medicine_id FK
        int dosage_id FK
        date start_date
        date end_date
    }
    medication_takes {
        int id PK
        int prescription_id FK
        datetime taken_at
    }

    users ||--o{ prescriptions : "has_many"
    medicines ||--o{ dosages : "has_many"
    medicines ||--o{ prescriptions : "has_many"
    dosages ||--o{ prescriptions : "has_many"
    prescriptions ||--o{ medication_takes : "has_many"
}
