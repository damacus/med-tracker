---
description: Get existing spec working
auto_execution_mode: 1
---

- Run `bundle exec rspec --format progress`
- Identify existing errors
- Batch fix
- When fixing make sure that we follow the rubocop stlye guidelines
- Make sure we do not break existing tests
- If there are tests missing create new ones

# Defintion of done

- run `--format progress`
  The command output should contain the string "0 failures"
