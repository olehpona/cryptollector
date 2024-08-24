docker buildx build . -t ghcr.io/olehpona/paymenator-dev:latest
docker push ghcr.io/olehpona/paymenator-dev:latest
docker image rm ghcr.io/olehpona/paymenator-dev:latest
