docker buildx build --platform linux/amd64,linux/arm64 -f deploy/Dockerfile -t jhash14/connections-web:latest --push .
