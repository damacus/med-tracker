function ipv4_number -a address
    set -l octets (string split . -- $address)
    test (count $octets) -eq 4
    or return 2
    for octet in $octets
        string match -rq '^[0-9]{1,3}$' -- $octet
        or return 2
        test $octet -le 255
        or return 2
    end
    math -s0 "((($octets[1] * 256 + $octets[2]) * 256 + $octets[3]) * 256 + $octets[4])"
end

function cidr_bounds -a subnet
    set -l parts (string split / -- $subnet)
    test (count $parts) -eq 2
    or return 2
    string match -rq '^[0-9]{1,2}$' -- $parts[2]
    or return 2
    test $parts[2] -le 32
    or return 2
    set -l address_number (ipv4_number $parts[1])
    or return 2
    set -l block_size (math -s0 "2 ^ (32 - $parts[2])")
    set -l first (math -s0 "floor($address_number / $block_size) * $block_size")
    set -l last (math -s0 "$first + $block_size - 1")
    echo $first
    echo $last
end

set -l candidate $argv[1]
string match -rq '^([0-9]{1,3}\.){3}[0-9]{1,3}/28$' -- $candidate
or begin; echo 'Contract test subnet must be an IPv4 /28 network' >&2; exit 2; end
set -l address (string split / -- $candidate)[1]
set -l candidate_bounds (cidr_bounds $candidate)
or begin; echo 'Invalid contract test subnet' >&2; exit 2; end
test (ipv4_number $address) -eq $candidate_bounds[1]
or begin; echo 'Contract test subnet must use its network address' >&2; exit 2; end
test (uname) = Darwin
or begin; echo 'Contract test subnet host-route check requires macOS' >&2; exit 2; end

set -l candidate_route (route -n get -inet $address | string match -r '^\s*(?:gateway|interface):\s*.*')
or begin; echo 'Could not inspect host route for contract subnet' >&2; exit 2; end
set -l default_route (route -n get -inet 8.8.8.8 | string match -r '^\s*(?:gateway|interface):\s*.*')
or begin; echo 'Could not inspect default host route' >&2; exit 2; end
set -l candidate_route_text (string join , -- $candidate_route)
set -l default_route_text (string join , -- $default_route)
test "$candidate_route_text" = "$default_route_text"
or begin; echo "Contract test subnet overlaps a host route: $candidate" >&2; exit 2; end

set -l network_ids (docker network ls -q)
or exit $status
if test (count $network_ids) -gt 0
    set -l existing_subnets (docker network inspect $network_ids | jq -r '.[].IPAM.Config[]?.Subnet // empty')
    or exit $status
    for existing in $existing_subnets
        set -l existing_bounds (cidr_bounds $existing)
        or continue
        if test $candidate_bounds[1] -le $existing_bounds[2]; and test $existing_bounds[1] -le $candidate_bounds[2]
            echo "Contract test subnet $candidate overlaps Docker subnet $existing" >&2
            exit 2
        end
    end
end
