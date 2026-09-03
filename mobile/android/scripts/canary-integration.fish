#!/usr/bin/env fish

for variable_name in MEDTRACKER_CANARY_BASE_URL MEDTRACKER_CANARY_EMAIL MEDTRACKER_CANARY_PASSWORD
    if not set -q $variable_name
        echo "$variable_name is required" >&2
        exit 2
    end
    if test -z "$$variable_name"
        echo "$variable_name is required" >&2
        exit 2
    end
end

exec ./gradlew :phone:testStagingCanaryIntegrationUnitTest -Pmedtracker.canaryIntegration=true --no-daemon
