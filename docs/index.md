# MedTracker Documentation

Welcome to the MedTracker documentation! This documentation covers the
design, architecture, and usage of the MedTracker medication tracking
application.

## What is MedTracker?

MedTracker is a Ruby on Rails application designed to help users monitor
their medication intake, ensuring they adhere to their prescribed schedules
and restrictions. It provides a clear interface for managing prescriptions
and logging doses, with built-in validations to prevent common mistakes
like taking too much medication or taking doses too close together.

## Key Features

- **Prescription Management:** Create and manage prescriptions with details
  like dosage, frequency, and start/end dates
- **Dose Tracking:** Easily log each dose of medication taken
- **Timing Restrictions:** Set rules for maximum daily doses and minimum
  hours between doses to prevent accidental overdose
- **Dose Cycles:** Supports daily, weekly, and monthly dosing schedules
- **Active/Inactive Prescriptions:** Automatically tracks which prescriptions
  are currently active
- **Validation:** Smart validations to ensure doses are taken according to
  prescription rules

## Documentation Structure

This documentation is organized into several sections:

- **[Design & Architecture](design.md)** - Overview of the application
  architecture, data models, and technical decisions
- **[User Management](user-management.md)** - Comprehensive guide to the
  user management system, including roles, permissions, and capacity
  management

## Quick Start

For setup instructions and development information, please see the
[README](https://github.com/damacus/med-tracker/blob/main/README.md)
in the main repository.

## Contributing

Interested in contributing? Check out the
[Contributing Guide](https://github.com/damacus/med-tracker/blob/main/CONTRIBUTING.md)
for information on how to get started.

## GitHub Copilot Agent

This project includes a documentation agent that can help identify
opportunities to improve and expand this documentation. See the
[agents documentation](https://github.com/damacus/med-tracker/blob/main/.github/agents/README.md)
for more information.
