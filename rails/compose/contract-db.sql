SELECT 'CREATE DATABASE medtracker_contract'
WHERE NOT EXISTS (SELECT 1 FROM pg_database WHERE datname = 'medtracker_contract')
\gexec
