build-docker-images:
    docker build --platform linux/amd64 -t sp1-tee-private-proving:server --target server .
    docker build --platform linux/amd64 -t sp1-tee-private-proving:fulfiller --target fulfiller .

publish-docker-images:
    docker build -t public.ecr.aws/succinct-labs/sp1-tee-private-proving:server --target server .
    docker build -t public.ecr.aws/succinct-labs/sp1-tee-private-proving:fulfiller --target fulfiller .
    docker push public.ecr.aws/succinct-labs/sp1-tee-private-proving:server
    docker push public.ecr.aws/succinct-labs/sp1-tee-private-proving:fulfiller

pull-docker-images:
    docker pull public.ecr.aws/succinct-labs/sp1-tee-private-proving:server
    docker pull public.ecr.aws/succinct-labs/sp1-tee-private-proving:fulfiller

show-digests:
    docker inspect sp1-tee-private-proving:server sp1-tee-private-proving:fulfiller --format "{{{{.RepoTags}}: {{{{.RepoDigests}}"

get-attestation:
    cargo r --bin sp1-tee-app-integrity-verifier

get-and-verify-quote:
    curl -sO https://tee.sp1-lumiere.xyz/evidences/quote.json
    docker run -v "./quote.json:/quote.json" dstacktee/dstack-verifier:0.5.4 --verify "/quote.json"

retrieve-docker-compose app_id:
    curl -s https://{{app_id}}-8090.succinct.phala.network/prpc/Info | jq -j .tcb_info | jq -j .app_compose | jq -j .docker_compose_file

verify-compose-hash app_id:
    curl -s https://{{app_id}}-8090.succinct.phala.network/prpc/Info | jq -j .tcb_info | jq -j .app_compose | sha256sum
