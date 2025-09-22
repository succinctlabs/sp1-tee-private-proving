build-docker-images:
    docker build -t public.ecr.aws/succinct-labs/sp1-tee-private-proving:server --target server .
    docker build -t public.ecr.aws/succinct-labs/sp1-tee-private-proving:fulfiller --target fulfiller .

publish-docker-images:
    docker push public.ecr.aws/succinct-labs/sp1-tee-private-proving:server
    docker push public.ecr.aws/succinct-labs/sp1-tee-private-proving:fulfiller

show-digests:
    docker images public.ecr.aws/succinct-labs/sp1-tee-private-proving --format "{{{{.Tag}}: {{{{.Digest}}"

get-attestation:
    cargo r --bin sp1-tee-app-integrity-verifier