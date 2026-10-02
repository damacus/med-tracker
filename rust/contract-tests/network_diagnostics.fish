set -l network_ids (docker network ls -q)
test (count $network_ids) -gt 0
or exit 0
docker network inspect $network_ids | jq -r '.[] | [.Name, (.IPAM.Config[0].Subnet // ""), (.Labels["com.docker.compose.project"] // "")] | @tsv'
netstat -rn -f inet
