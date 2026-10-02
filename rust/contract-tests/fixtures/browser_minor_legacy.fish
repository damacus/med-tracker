function main -a project browser_files fixture_path
    rtk proxy task api:contract-browser-node "CONTRACT_PROJECT=$project" "BROWSER_TEST_FILES=$browser_files"
    return $status
end

main $argv

