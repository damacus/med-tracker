# MedTracker Product Overview

MedTracker is a Ruby on Rails application designed to help users manage and track their medication schedules effectively.

## Core Purpose

- Help users monitor medication intake and adhere to prescribed schedules
- Prevent common medication errors like overdosing or taking doses too close together
- Provide clear interface for managing prescriptions and logging doses

## Key Features

- **Prescription Management**: Create and manage prescriptions with dosage, frequency, and date ranges
- **Dose Tracking**: Log each medication dose taken with timestamps
- **Safety Validations**: Enforce maximum daily doses and minimum hours between doses
- **Flexible Scheduling**: Support daily, weekly, and monthly dosing cycles
- **User Management**: Role-based access (admin, user, child, carer) with appropriate permissions
- **Active/Inactive Tracking**: Automatically manage prescription status

## Target Users

- Individuals managing their own medications
- Caregivers managing medications for family members
- Healthcare providers needing medication adherence tracking

## Data Models

- **Users**: Authentication and role management
- **People**: Individuals receiving medication (can be different from users)
- **Medicines**: Medication catalog with dosage info and warnings
- **Prescriptions**: Links people to medicines with specific instructions
- **Medication Takes**: Records of actual doses administered
